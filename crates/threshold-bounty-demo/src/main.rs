//! Reproducible Esmeralda-only bounty transactions. Secrets stay in the parent lab's .local/.
use anyhow::{Context, Result, ensure};
use ootle_byte_type::ToByteType;
use ootle_rs::{
    Network, ToAccountAddress,
    builtin_templates::{UnsignedTransactionBuilder, account::IAccount, faucet::IFaucet},
    key_provider::PrivateKeyProvider,
    keys::OotleSecretKey,
    provider::{IndexerProvider, Provider, ProviderBuilder, WantInput},
    transaction::TransactionSigner,
    wallet::OotleWallet,
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use tari_crypto::{
    keys::{PublicKey, SecretKey},
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
};
use tari_engine_types::{substate::SubstateId, transaction_receipt::TransactionReceipt};
use tari_ootle_transaction::{Epoch, Transaction, TransactionBuilder, UnsignedTransaction, args};
use tari_ootle_wallet_crypto::{
    MaskAndValue, StealthInputWitness, StealthOutputWitness,
    encryption::{decrypt_with_password, encrypt_with_password},
    stealth,
};
use tari_template_lib::{
    prelude::{
        Amount, ComponentAddress, Hash32, RistrettoPublicKeyBytes, TemplateAddress, UtxoAddress,
    },
    types::constants::TARI_TOKEN,
};
use tari_utilities::ByteArray;
use threshold_bounty_template::{AwardInput, Tier};
use threshold_client::{check_aggregate, escrow_input, private_output};

type Wallet = OotleWallet;
const INDEXER: &str = "https://ootle-indexer-a.tari.com";
const TIER: u64 = 15_000_000_000;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn private_root() -> PathBuf {
    repo_root().parent().unwrap().join(".local")
}
fn evidence_path(label: &str, suffix: &str) -> PathBuf {
    let base =
        if suffix == "dry-run" || label.starts_with("faucet-") || label.starts_with("transfer-") {
            private_root().join("bounty-evidence")
        } else {
            repo_root().join("evidence").join("bounty")
        };
    base.join(format!("{label}-{suffix}.json"))
}
fn private_path(name: &str) -> PathBuf {
    private_root().join(format!("bounty-{name}.recovery"))
}
fn unlock() -> Result<Vec<u8>> {
    Ok(fs::read(private_root().join("unlock.secret"))?)
}
fn save_private(name: &str, value: &Value) -> Result<()> {
    fs::create_dir_all(private_root())?;
    fs::write(
        private_path(name),
        encrypt_with_password(&serde_json::to_vec(value)?, &unlock()?)?,
    )?;
    Ok(())
}
fn read_private(name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&decrypt_with_password(
        &fs::read(private_path(name))?,
        &unlock()?,
    )?)?)
}
fn identity(name: &str) -> Result<PrivateKeyProvider> {
    ensure!(
        name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "invalid identity"
    );
    fs::create_dir_all(private_root())?;
    if !private_root().join("unlock.secret").exists() {
        fs::write(
            private_root().join("unlock.secret"),
            RistrettoSecretKey::random(&mut rand::rng()).as_bytes(),
        )?;
    }
    if !private_path(name).exists() {
        let secret = OotleSecretKey::random(Network::Esmeralda);
        save_private(
            name,
            &json!({
                "version": 1, "network": "esmeralda",
                "account": hex::encode(secret.account_secret().as_bytes()),
                "view": hex::encode(secret.view_only_secret().as_bytes())
            }),
        )?;
    }
    let data = read_private(name)?;
    ensure!(
        data["version"] == 1 && data["network"] == "esmeralda",
        "wrong identity context"
    );
    let account = RistrettoSecretKey::from_canonical_bytes(&hex::decode(
        data["account"].as_str().context("missing account")?,
    )?)
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let view = RistrettoSecretKey::from_canonical_bytes(&hex::decode(
        data["view"].as_str().context("missing view")?,
    )?)
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(PrivateKeyProvider::new(OotleSecretKey::new(
        Network::Esmeralda,
        account,
        view,
    )))
}
async fn provider(signer: &PrivateKeyProvider) -> Result<IndexerProvider<Wallet>> {
    let p = ProviderBuilder::new()
        .wallet(OotleWallet::from(signer.clone()))
        .connect(INDEXER)
        .await?;
    ensure!(
        p.get_network().await? == Network::Esmeralda,
        "wrong network"
    );
    Ok(p)
}
fn receipt(label: &str) -> Result<Option<TransactionReceipt>> {
    let path = evidence_path(label, "receipt");
    if !path.exists() {
        return Ok(None);
    }
    let data: Value = serde_json::from_slice(&fs::read(path)?)?;
    let r: TransactionReceipt = serde_json::from_value(data["receipt"].clone())?;
    ensure!(
        r.outcome.is_commit(),
        "{label} saved receipt is not a full main-intent commit"
    );
    Ok(Some(r))
}
fn txid(label: &str) -> Result<String> {
    let data: Value = serde_json::from_slice(&fs::read(evidence_path(label, "receipt"))?)?;
    Ok(data["transaction_id"]
        .as_str()
        .context("missing transaction ID")?
        .into())
}
fn sign(unsigned: UnsignedTransaction, signer: &PrivateKeyProvider) -> Transaction {
    let secret = signer.credentials().account_secret();
    let key = RistrettoPublicKey::from_secret_key(secret).to_byte_type();
    unsigned.add_signer(&key, secret).seal(secret)
}
async fn submit(
    p: &mut IndexerProvider<Wallet>,
    signer: &PrivateKeyProvider,
    unsigned: UnsignedTransaction,
    label: &str,
) -> Result<TransactionReceipt> {
    if let Some(r) = receipt(label)? {
        return Ok(r);
    }
    fs::create_dir_all(repo_root().join("evidence").join("bounty"))?;
    fs::create_dir_all(private_root().join("bounty-evidence"))?;
    let pending_path = private_root().join(format!("bounty-{label}-pending.json"));
    if pending_path.exists() {
        let data: Value = serde_json::from_slice(&fs::read(&pending_path)?)?;
        let id = tari_ootle_transaction::TransactionId::from_hex(
            data["transaction_id"]
                .as_str()
                .context("bad pending record")?,
        )?;
        let client = p
            .weak_client()
            .upgrade()
            .context("indexer connection lost")?;
        let response = client
            .get_transaction_receipt(id.into_receipt_address())
            .await?;
        ensure!(
            response.receipt.outcome.is_commit(),
            "{label} pending main intent did not commit"
        );
        fs::write(
            evidence_path(label, "receipt"),
            serde_json::to_vec_pretty(
                &json!({"transaction_id": id.to_string(), "receipt": response.receipt}),
            )?,
        )?;
        return receipt(label)?.context("receipt reconciliation failed");
    }
    let dry = p
        .send_dry_run(sign(unsigned.clone().with_dry_run(true), signer))
        .await?;
    fs::write(
        evidence_path(label, "dry-run"),
        serde_json::to_vec_pretty(&dry)?,
    )?;
    ensure!(
        dry.finalize.any_reject().is_none(),
        "{label} dry run rejected: {}",
        dry.finalize.result
    );
    let transaction = sign(unsigned, signer);
    let expected = transaction.calculate_id();
    fs::write(
        &pending_path,
        serde_json::to_vec_pretty(
            &json!({"transaction_id": expected.to_string(), "status": "prepared-or-submitted"}),
        )?,
    )?;
    let pending = p.send_transaction(transaction).await?;
    ensure!(pending.tx_id() == expected, "transaction ID changed");
    let outcome = pending.watch().await?;
    println!("{label}: {} {outcome}", pending.tx_id());
    ensure!(outcome.is_commit(), "{label} main intent did not commit");
    let r = pending.get_receipt().await?;
    ensure!(
        r.outcome.is_commit(),
        "{label} receipt is not a full main-intent commit"
    );
    fs::write(
        evidence_path(label, "receipt"),
        serde_json::to_vec_pretty(&json!({"transaction_id": expected.to_string(), "receipt": r}))?,
    )?;
    Ok(r)
}
async fn base(p: &IndexerProvider<Wallet>) -> Result<TransactionBuilder> {
    Ok(Transaction::builder(
        Network::Esmeralda,
        Epoch(p.get_epoch().await?.as_u64() + 10),
    )
    .with_auto_fill_inputs()
    .pay_fee_from_component(
        p.default_signer_address().to_account_address(),
        5_000_000u64,
    ))
}
async fn execute(
    p: &mut IndexerProvider<Wallet>,
    signer: &PrivateKeyProvider,
    builder: TransactionBuilder,
    label: &str,
) -> Result<TransactionReceipt> {
    execute_with_accounts(p, signer, builder, label, &[]).await
}
async fn execute_with_accounts(
    p: &mut IndexerProvider<Wallet>,
    signer: &PrivateKeyProvider,
    builder: TransactionBuilder,
    label: &str,
    extra_accounts: &[ComponentAddress],
) -> Result<TransactionReceipt> {
    if let Some(r) = receipt(label)? {
        return Ok(r);
    }
    let mut wants = HashSet::from([WantInput::VaultForResource {
        component_address: p.default_signer_address().to_account_address(),
        resource_address: TARI_TOKEN,
        required: true,
    }]);
    for account in extra_accounts {
        wants.insert(WantInput::VaultForResource {
            component_address: *account,
            resource_address: TARI_TOKEN,
            required: true,
        });
    }
    let unsigned = p
        .resolve_input_want_list(builder.build_unsigned(), &wants)
        .await?;
    submit(p, signer, unsigned, label).await
}
async fn faucet(name: &str, signer: &PrivateKeyProvider) -> Result<()> {
    let mut p = provider(signer).await?;
    let label = format!("faucet-{name}");
    if receipt(&label)?.is_some() {
        return Ok(());
    }
    let max = Epoch(p.get_epoch().await?.as_u64() + 10);
    let unsigned = IFaucet::new(&p, max)
        .take_faucet_funds()
        .pay_fee(1_000_000u64)
        .prepare()
        .await?;
    submit(&mut p, signer, unsigned, &label).await?;
    Ok(())
}
fn created_component(r: &TransactionReceipt) -> Result<ComponentAddress> {
    r.diff_summary
        .upped
        .iter()
        .find_map(|s| {
            if s.version == 0 {
                s.substate_id.as_component_address()
            } else {
                None
            }
        })
        .context("created component missing")
}
fn published_template(r: &TransactionReceipt) -> Result<TemplateAddress> {
    r.diff_summary
        .upped
        .iter()
        .find_map(|s| s.substate_id.as_template().map(|a| a.as_template_address()))
        .context("published template missing")
}
fn key(signer: &PrivateKeyProvider) -> RistrettoPublicKeyBytes {
    *signer.address().account_public_key()
}
fn hash(n: u8) -> Hash32 {
    Hash32::from_array([n; 32])
}

async fn create_bounty(
    p: &mut IndexerProvider<Wallet>,
    manager: &PrivateKeyProvider,
    template: TemplateAddress,
    treasury_key: RistrettoPublicKeyBytes,
    issue: u64,
    label: &str,
) -> Result<ComponentAddress> {
    if let Some(r) = receipt(label)? {
        return created_component(&r);
    }
    let b = base(p).await?.call_function(
        template,
        "new",
        args![
            String::from("tari-project/tari-ootle"),
            issue,
            hash((issue % 200) as u8 + 1),
            Tier::S,
            treasury_key,
            vec![key(manager)],
            1u32,
            Vec::<RistrettoPublicKeyBytes>::new(),
            0u32
        ],
    );
    created_component(&execute(p, manager, b, label).await?)
}
fn save_cell(name: &str, cell: &MaskAndValue) -> Result<()> {
    save_private(
        name,
        &json!({"value": cell.value, "mask": hex::encode(cell.mask.as_bytes())}),
    )
}
fn load_cell(name: &str) -> Result<MaskAndValue> {
    let data = read_private(name)?;
    let mask = RistrettoSecretKey::from_canonical_bytes(&hex::decode(
        data["mask"].as_str().context("missing cell mask")?,
    )?)
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(MaskAndValue {
        value: data["value"].as_u64().context("missing cell value")?,
        mask,
    })
}
fn escrow_source(cell: &MaskAndValue) -> SubstateId {
    SubstateId::Utxo(UtxoAddress::new(
        TARI_TOKEN,
        cell.to_commitment().to_byte_type().into(),
    ))
}
async fn fund(
    p: &mut IndexerProvider<Wallet>,
    treasury: &PrivateKeyProvider,
    component: ComponentAddress,
    label: &str,
    cell_name: &str,
) -> Result<MaskAndValue> {
    if receipt(label)?.is_some() {
        return load_cell(cell_name);
    }
    let statement_name = format!("{cell_name}-statement");
    if !private_path(&statement_name).exists() {
        ensure!(
            !private_path(cell_name).exists(),
            "cell saved without funding statement; manual recovery required"
        );
        let output = private_output(treasury.address(), TIER, Some(component))?;
        let cell = MaskAndValue {
            value: output.witness.amount,
            mask: output.witness.mask.clone(),
        };
        save_cell(cell_name, &cell)?;
        let statement = stealth::create_transfer_statement(
            Vec::<StealthInputWitness>::new(),
            Amount::from(TIER),
            [&output],
            Amount::zero(),
        )?;
        save_private(&statement_name, &serde_json::to_value(&statement)?)?;
    }
    let cell = load_cell(cell_name)?;
    let statement: tari_template_lib::prelude::StealthTransferStatement =
        serde_json::from_value(read_private(&statement_name)?)?;
    let mask = check_aggregate(&[cell.clone()], TIER)?;
    let b = base(p)
        .await?
        .call_method(
            treasury.address().to_account_address(),
            "withdraw",
            args![TARI_TOKEN, TIER],
        )
        .put_last_instruction_output_on_workspace("funds")
        .call_method(
            component,
            "fund",
            args![Workspace("funds"), statement, mask],
        );
    execute(p, treasury, b, label).await?;
    Ok(cell)
}

async fn collect_test_funds(treasury: &PrivateKeyProvider) -> Result<()> {
    let treasury_provider = provider(treasury).await?;
    let treasury_account = treasury.address().to_account_address();
    let needed = Amount::from(TIER + 100_000_000);
    if treasury_provider
        .get_account_balance(treasury_account, TARI_TOKEN)
        .await?
        >= needed
    {
        return Ok(());
    }
    for index in 1..=80 {
        let name = format!("donor-{index:02}");
        if receipt(&format!("transfer-{name}"))?.is_some() {
            continue;
        }
        let donor = identity(&name)?;
        faucet(&name, &donor).await?;
        let mut p = provider(&donor).await?;
        let label = format!("transfer-{name}");
        let b = base(&p)
            .await?
            .call_method(
                donor.address().to_account_address(),
                "withdraw",
                args![TARI_TOKEN, 990_000_000u64],
            )
            .put_last_instruction_output_on_workspace("donation")
            .call_method(treasury_account, "deposit", args![Workspace("donation")]);
        execute_with_accounts(&mut p, &donor, b, &label, &[treasury_account]).await?;
        if treasury_provider
            .get_account_balance(treasury_account, TARI_TOKEN)
            .await?
            >= needed
        {
            return Ok(());
        }
    }
    anyhow::bail!("demo treasury still lacks the exact 15,000 tTARI tier after faucet aggregation")
}

#[tokio::main]
async fn main() -> Result<()> {
    let treasury = identity("treasury")?;
    let manager = identity("council-1")?;
    let reviewer_two = identity("council-2")?;
    let reviewer_three = identity("council-3")?;
    let recipient = identity("recipient")?;
    for (name, signer) in [
        ("treasury", &treasury),
        ("council-1", &manager),
        ("council-2", &reviewer_two),
        ("recipient", &recipient),
    ] {
        faucet(name, signer).await?;
    }
    let mut tp = provider(&treasury).await?;
    let template = if let Some(r) = receipt("publish")? {
        published_template(&r)?
    } else {
        let target = std::env::var("CARGO_TARGET_DIR")
            .unwrap_or_else(|_| repo_root().join("target").to_string_lossy().into_owned());
        let code = fs::read(
            Path::new(&target)
                .join("wasm32-unknown-unknown/release/threshold_bounty_template.wasm"),
        )?;
        let max = Epoch(tp.get_epoch().await?.as_u64() + 10);
        let unsigned = IAccount::new(&tp, max)
            .pay_fee(100_000_000u64)
            .publish_template(code)
            .prepare()
            .await?;
        published_template(&submit(&mut tp, &treasury, unsigned, "publish").await?)?
    };
    if receipt("one-fund")?.is_none() {
        collect_test_funds(&treasury).await?;
    }
    let mut mp = provider(&manager).await?;
    let one = create_bounty(
        &mut mp,
        &manager,
        template,
        key(&treasury),
        900001,
        "one-create",
    )
    .await?;
    let one_cell = fund(&mut tp, &treasury, one, "one-fund", "one-cell").await?;
    let b = base(&mp)
        .await?
        .call_method(one, "mark_no_work", args![hash(91)]);
    execute(&mut mp, &manager, b, "one-no-work").await?;
    if receipt("one-refund")?.is_none() {
        let transfer = stealth::create_transfer_statement(
            [escrow_input(one_cell.clone(), one)?],
            Amount::zero(),
            std::iter::empty::<&StealthOutputWitness>(),
            Amount::from(TIER),
        )?;
        let b = base(&tp)
            .await?
            .add_input(escrow_source(&one_cell))
            .call_method(one, "refund_no_work", args![transfer])
            .put_last_instruction_output_on_workspace("refund")
            .call_method(
                treasury.address().to_account_address(),
                "deposit",
                args![Workspace("refund")],
            );
        execute(&mut tp, &treasury, b, "one-refund").await?;
    }
    let two = create_bounty(
        &mut mp,
        &manager,
        template,
        key(&treasury),
        900002,
        "two-create",
    )
    .await?;
    let b = base(&mp).await?.call_method(
        two,
        "set_reviewers",
        args![
            vec![key(&manager), key(&reviewer_two), key(&reviewer_three)],
            2u32
        ],
    );
    execute(&mut mp, &manager, b, "two-set-reviewers").await?;
    let two_cell = fund(&mut tp, &treasury, two, "two-fund", "two-cell").await?;
    let b = base(&mp).await?.call_method(
        two,
        "propose_award",
        args![
            vec![AwardInput {
                recipient: key(&recipient),
                amount: TIER,
                pr_hash: hash(92)
            }],
            hash(93)
        ],
    );
    execute(&mut mp, &manager, b, "two-propose").await?;
    let mut rp = provider(&reviewer_two).await?;
    let b = base(&rp).await?.call_method(two, "approve_award", args![]);
    execute(&mut rp, &reviewer_two, b, "two-approve").await?;
    let mut wp = provider(&recipient).await?;
    if receipt("two-claim")?.is_none() {
        let transfer = stealth::create_transfer_statement(
            [escrow_input(two_cell.clone(), two)?],
            Amount::zero(),
            std::iter::empty::<&StealthOutputWitness>(),
            Amount::from(TIER),
        )?;
        let b = base(&wp)
            .await?
            .add_input(escrow_source(&two_cell))
            .call_method(two, "claim", args![0u32, transfer])
            .put_last_instruction_output_on_workspace("payout")
            .call_method(
                recipient.address().to_account_address(),
                "deposit",
                args![Workspace("payout")],
            );
        execute(&mut wp, &recipient, b, "two-claim").await?;
    }
    let three = create_bounty(
        &mut mp,
        &manager,
        template,
        key(&treasury),
        900003,
        "three-create",
    )
    .await?;
    let b = base(&mp).await?.call_method(
        three,
        "set_reviewers",
        args![
            vec![key(&manager), key(&reviewer_two), key(&reviewer_three)],
            1u32
        ],
    );
    execute(&mut mp, &manager, b, "three-set-reviewers").await?;
    if receipt("three-fund")?.is_none() {
        collect_test_funds(&treasury).await?;
    }
    fund(&mut tp, &treasury, three, "three-fund", "three-cell").await?;
    let four = create_bounty(
        &mut mp,
        &manager,
        template,
        key(&treasury),
        900004,
        "four-create",
    )
    .await?;
    if receipt("four-fund")?.is_none() {
        collect_test_funds(&treasury).await?;
    }
    fund(&mut tp, &treasury, four, "four-fund", "four-cell").await?;
    let five = create_bounty(
        &mut mp,
        &manager,
        template,
        key(&treasury),
        900005,
        "five-create",
    )
    .await?;
    let b = base(&mp).await?.call_method(
        five,
        "set_reviewers",
        args![
            vec![key(&manager), key(&reviewer_two), key(&reviewer_three)],
            2u32
        ],
    );
    execute(&mut mp, &manager, b, "five-set-reviewers").await?;
    if receipt("five-fund")?.is_none() {
        collect_test_funds(&treasury).await?;
    }
    fund(&mut tp, &treasury, five, "five-fund", "five-cell").await?;
    fs::write(
        repo_root().join("evidence/bounty/demo-index.json"),
        serde_json::to_vec_pretty(&json!({
            "network": "esmeralda", "indexer": INDEXER, "template": template.to_string(),
            "identity_note": "All council and treasury labels are demonstration keys, not Tari-designated identities.",
            "people": [
                {"name": "Demo Tari treasury", "key": key(&treasury).to_string()},
                {"name": "Demo council member 1", "key": key(&manager).to_string()},
                {"name": "Demo council member 2", "key": key(&reviewer_two).to_string()},
                {"name": "Demo council member 3", "key": key(&reviewer_three).to_string()},
                {"name": "Demo contributor", "key": key(&recipient).to_string()}
            ],
            "bounties": [
                {"issue": 900001, "component": one.to_string(), "reviewers": [key(&manager).to_string()], "quorum": 1,
                 "transactions": {"create": txid("one-create")?, "fund": txid("one-fund")?, "noWork": txid("one-no-work")?, "refund": txid("one-refund")?}},
                {"issue": 900002, "component": two.to_string(), "reviewers": [key(&manager).to_string(), key(&reviewer_two).to_string(), key(&reviewer_three).to_string()], "quorum": 2,
             "transactions": {"create": txid("two-create")?, "setReviewers": txid("two-set-reviewers")?, "fund": txid("two-fund")?, "propose": txid("two-propose")?, "approve": txid("two-approve")?, "claim": txid("two-claim")?}},
            {"issue": 900003, "component": three.to_string(), "reviewers": [key(&manager).to_string(), key(&reviewer_two).to_string(), key(&reviewer_three).to_string()], "quorum": 1,
             "transactions": {"create": txid("three-create")?, "setReviewers": txid("three-set-reviewers")?, "fund": txid("three-fund")?}},
            {"issue": 900004, "component": four.to_string(), "reviewers": [key(&manager).to_string()], "quorum": 1,
             "transactions": {"create": txid("four-create")?, "fund": txid("four-fund")?}},
            {"issue": 900005, "component": five.to_string(), "reviewers": [key(&manager).to_string(), key(&reviewer_two).to_string(), key(&reviewer_three).to_string()], "quorum": 2,
             "transactions": {"create": txid("five-create")?, "setReviewers": txid("five-set-reviewers")?, "fund": txid("five-fund")?}}
            ]
        }))?,
    )?;
    println!("Live bounty proof recorded in evidence/bounty/demo-index.json");
    Ok(())
}
