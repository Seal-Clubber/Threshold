//! Esmeralda integration demonstration. No backend or local ledger emulator.
use super::*;
use std::{collections::HashSet, str::FromStr};
use ootle_rs::{provider::{Provider, WantInput}, transaction::TransactionSigner};
use ootle_byte_type::{ToByteType, ConvertFromByteType};
use tari_crypto::{keys::PublicKey, ristretto::RistrettoPublicKey};
use tari_template_lib::{prelude::{Amount, Hash32, Scalar32Bytes, TemplateAddress, ComponentAddress, UtxoAddress}, types::{constants::TARI_TOKEN, SubstateOwnerRule}};
use tari_engine_types::substate::SubstateId;
use tari_ootle_transaction::{Transaction, TransactionBuilder, UnsignedTransaction, args};
use tari_ootle_wallet_crypto::{MaskAndValue, StealthInputWitness, stealth, kdfs};
use threshold_client::{private_output, private_transfer, decrypt_output, escrow_input, check_aggregate};
use threshold_template::{Terms, Stage, Pledge, escrow_root};
use tari_engine_types::transaction_receipt::TransactionReceipt;
use serde::{Serialize, Deserialize};

fn save_private<T: Serialize>(name: &str, value: &T) -> Result<()> {
    fs::write(format!(".local/{name}.recovery"), encrypt_with_password(&serde_json::to_vec(value)?, &fs::read(".local/unlock.secret")?)?)?;
    Ok(())
}
fn read_private<T: serde::de::DeserializeOwned>(name: &str) -> Result<T> {
    Ok(serde_json::from_slice(&decrypt_with_password(&fs::read(format!(".local/{name}.recovery"))?, &fs::read(".local/unlock.secret")?)?)?)
}
fn recorded_receipt(name: &str) -> Result<TransactionReceipt> {
    let v: serde_json::Value=serde_json::from_slice(&fs::read(format!("evidence/{name}-receipt.json"))?)?;
    Ok(serde_json::from_value(v["receipt"].clone())?)
}
fn receipt(name: &str) -> Result<TransactionReceipt> {
    let r=recorded_receipt(name)?;
    ensure!(r.outcome.is_commit(),"{name} saved receipt is not a full main-intent Commit");
    Ok(r)
}
fn done(name: &str) -> bool { Path::new(&format!("evidence/{name}-receipt.json")).exists() }

async fn base(p: &IndexerProvider<Wallet>) -> Result<TransactionBuilder> {
    Ok(Transaction::builder(Network::Esmeralda, Epoch(p.get_epoch().await?.as_u64()+10))
        .with_auto_fill_inputs().pay_fee_from_component(p.default_signer_address().to_account_address(), 5_000_000u64))
}
/// A method argument contains commitments, not tagged UTXO addresses. Ootle's
/// builder cannot infer the required input substates from that CBOR argument.
fn with_stealth_inputs(mut b: TransactionBuilder, transfer: &tari_template_lib::prelude::StealthTransferStatement) -> TransactionBuilder {
    for input in transfer.stealth_inputs() {
        b = b.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN, input.commitment.into())));
    }
    b
}
async fn resolve(p: &IndexerProvider<Wallet>, b: TransactionBuilder) -> Result<UnsignedTransaction> {
    let wants=HashSet::from([WantInput::VaultForResource { component_address:p.default_signer_address().to_account_address(),resource_address:TARI_TOKEN,required:true }]);
    Ok(p.resolve_input_want_list(b.build_unsigned(), &wants).await?)
}
fn sign(unsigned: UnsignedTransaction, signer: &PrivateKeyProvider, extras: &[RistrettoSecretKey]) -> Transaction {
    let secret=signer.credentials().account_secret();
    let pk=RistrettoPublicKey::from_secret_key(secret).to_byte_type();
    let mut tx=unsigned.add_signer(&pk,secret);
    for extra in extras { tx=tx.add_signer(&pk,extra); }
    tx.seal(secret)
}

async fn execute(p: &mut IndexerProvider<Wallet>, signer: &PrivateKeyProvider, b: TransactionBuilder, extra: &[RistrettoSecretKey], label: &str) -> Result<TransactionReceipt> {
    if done(label) { return receipt(label); }
    let pending_path=format!("evidence/{label}-pending.json");
    if Path::new(&pending_path).exists() {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(&pending_path)?)?;
        let id=tari_ootle_transaction::TransactionId::from_hex(value["transaction_id"].as_str().ok_or_else(||anyhow::anyhow!("bad pending record"))?)?;
        let client=p.weak_client().upgrade().ok_or_else(||anyhow::anyhow!("indexer connection lost"))?;
        let r=client.get_transaction_receipt(id.into_receipt_address()).await?;
        ensure!(r.receipt.outcome.is_commit(),"{label} main intent did not commit");
        fs::write(format!("evidence/{label}-receipt.json"),serde_json::to_vec_pretty(&json!({"transaction_id":id.to_string(),"receipt":&r.receipt}))?)?;
        return Ok(r.receipt);
    }
    let unsigned=resolve(p,b).await?;
    let dry=p.send_dry_run(sign(unsigned.clone().with_dry_run(true),signer,extra)).await?;
    fs::write(format!("evidence/{label}-dry-run.json"),serde_json::to_vec_pretty(&dry)?)?;
    ensure!(dry.finalize.any_reject().is_none(), "{label} rejected: {}",dry.finalize.result);
    let tx=sign(unsigned,signer,extra);
    let expected_id=tx.calculate_id();
    fs::write(&pending_path,serde_json::to_vec_pretty(&json!({"transaction_id":expected_id.to_string(),"status":"prepared-or-submitted"}))?)?;
    let pending=p.send_transaction(tx).await?;
    let txid=pending.tx_id().to_string();
    ensure!(pending.tx_id()==expected_id,"submitted transaction ID mismatch");
    let outcome=pending.watch().await?;
    println!("{label}: {txid} {outcome}");
    ensure!(outcome.is_commit(),"{label}: {outcome}");
    let r=pending.get_receipt().await?;
    ensure!(r.outcome.is_commit(),"{label} receipt is not a full main-intent Commit");
    fs::write(format!("evidence/{label}-receipt.json"),serde_json::to_vec_pretty(&json!({"transaction_id":txid,"receipt":r}))?)?;
    Ok(r)
}

async fn rejects(p: &IndexerProvider<Wallet>, signer: &PrivateKeyProvider, b: TransactionBuilder, label: &str, expected: &str) -> Result<()> {
    let unsigned=resolve(p,b).await?;
    let dry=p.send_dry_run(sign(unsigned.with_dry_run(true),signer,&[])).await?;
    fs::write(format!("evidence/{label}-dry-run.json"),serde_json::to_vec_pretty(&dry)?)?;
    let reason=dry.finalize.any_reject().ok_or_else(|| anyhow::anyhow!("attack unexpectedly accepted: {label}"))?.to_string();
    ensure!(reason.contains(expected),"{label}: wrong rejection: {reason}");
    println!("Rejected {label}: {expected}");
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
struct Deposit {
    shield: tari_template_lib::prelude::StealthTransferStatement,
    pledge: tari_template_lib::prelude::StealthTransferStatement,
    #[serde(with = "cell_encoding")]
    cells: Vec<MaskAndValue>,
    input_key: String,
}

/// All application-specific data needed by a fresh client. The encrypted file
/// and its separately stored unlock key can be moved off this machine.
#[derive(Serialize, Deserialize)]
struct RecoveryPackage {
    version: u32,
    network: String,
    indexer: String,
    template: String,
    component: String,
    terms_cbor: String,
    pledge_id: u32,
    account_secret: String,
    view_secret: String,
    deposit: Deposit,
    shield_txid: Option<String>,
    pledge_txid: Option<String>,
}

#[derive(minicbor::Decode)]
struct StateSnapshot {
    #[n(0)] terms: Terms,
    #[n(1)] pledges: Vec<Pledge>,
    #[n(2)] selected: Vec<u32>,
    #[n(3)] active: bool,
    #[n(4)] next_stage: u32,
    #[n(5)] _votes: Vec<Vec<Option<Hash32>>>,
}

fn recorded_txid(label: &str) -> Option<String> {
    let path = format!("evidence/{label}-receipt.json");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    value["transaction_id"].as_str().map(str::to_owned)
}

mod cell_encoding {
    use super::*;
    pub fn serialize<S: serde::Serializer>(cells: &[MaskAndValue], serializer:S) -> std::result::Result<S::Ok,S::Error> {
        cells.iter().map(|c|(c.value,hex::encode(c.mask.as_bytes()))).collect::<Vec<_>>().serialize(serializer)
    }
    pub fn deserialize<'de,D:serde::Deserializer<'de>>(deserializer:D) -> std::result::Result<Vec<MaskAndValue>,D::Error> {
        let values=Vec::<(u64,String)>::deserialize(deserializer)?;
        values.into_iter().map(|(value,mask)| {
            let bytes=hex::decode(mask).map_err(serde::de::Error::custom)?;
            let mask=RistrettoSecretKey::from_canonical_bytes(&bytes).map_err(serde::de::Error::custom)?;
            Ok(MaskAndValue{value,mask})
        }).collect()
    }
}

pub async fn run(p: &mut IndexerProvider<Wallet>, operator: &PrivateKeyProvider) -> Result<()> {
    let published=receipt("publish")?;
    let template=published.diff_summary.upped.iter().find_map(|s| s.substate_id.to_string().strip_prefix("template_").map(|a| a.to_string())).ok_or_else(|| anyhow::anyhow!("missing published template"))?;
    let template=TemplateAddress::from_str(&template)?;
    let roles=["alice","bob","carol","sponsor"];
    let mut signers=Vec::new(); let mut providers=Vec::new();
    for name in roles {
        let signer=identity_named(name)?;
        let mut provider=ProviderBuilder::new().wallet(OotleWallet::from(signer.clone())).connect(INDEXER).await?;
        if !done(&format!("faucet-{name}")) {
            let unsigned=IFaucet::new(&provider,Epoch(provider.get_epoch().await?.as_u64()+10)).take_faucet_funds().pay_fee(1_000_000u64).prepare().await?;
            submit(&mut provider,unsigned,&format!("faucet-{name}")).await?;
        }
        signers.push(signer); providers.push(provider);
    }
    let terms: Terms=if Path::new(".local/demo-terms.recovery").exists() {
        let bytes: String=read_private("demo-terms")?; tari_bor::decode(&hex::decode(bytes)?)?
    } else {
        let epoch=p.get_epoch().await?.as_u64();
        let terms=Terms { version:1, nonce:Hash32::from_array(rand::random()), resource:TARI_TOKEN,
            funding_deadline:epoch+20, sponsor_key:*signers[3].address().account_public_key(), stages:vec![
                Stage { leg:0,community:12_000_000,sponsor:8_000_000,recipient:*operator.address().account_public_key(),reviewers:vec![],quorum:0,deadline:epoch+21,upfront:true },
                Stage { leg:0,community:24_000_000,sponsor:16_000_000,recipient:*operator.address().account_public_key(),reviewers:vec![*signers[0].address().account_public_key(),*signers[1].address().account_public_key()],quorum:2,deadline:epoch+22,upfront:false },
                Stage { leg:1,community:24_000_000,sponsor:16_000_000,recipient:*operator.address().account_public_key(),reviewers:vec![*signers[0].address().account_public_key(),*signers[1].address().account_public_key()],quorum:2,deadline:epoch+23,upfront:false },
            ]};
        save_private("demo-terms",&hex::encode(tari_bor::encode(&terms)?))?; terms
    };
    let b=base(p).await?.call_function(template,"new",args![terms.clone()]);
    let created=execute(p,operator,b,&[],"campaign").await?;
    let component=created.diff_summary.upped.iter().find_map(|s| if s.version==0 {s.substate_id.as_component_address()} else {None}).ok_or_else(|| anyhow::anyhow!("campaign address missing"))?;
    fs::write("evidence/deployment.json",serde_json::to_vec_pretty(&json!({"network":"esmeralda","indexer":INDEXER,"template":template.to_string(),"component":component.to_string(),"live_budget_microtari":100_000_000,"scale":"1:100 of brief example","funding_deadline":terms.funding_deadline,"delivery_deadlines":terms.stages.iter().map(|s| s.deadline).collect::<Vec<_>>()}))?)?;
    let allocations=[[3_000_000,6_000_000,6_000_000],[4_000_000,8_000_000,8_000_000],[5_000_000,10_000_000,10_000_000],[8_000_000,16_000_000,16_000_000]];
    let mut deposits=Vec::new();
    for (i,name) in roles.iter().enumerate() {
        let path=format!("deposit-{name}");
        let deposit: Deposit=if Path::new(&format!(".local/{path}.recovery")).exists() {read_private(&path)?} else {
            let address=signers[i].address();
            let value=allocations[i].iter().sum::<u64>();
            let funding=private_output(address,value,None)?;
            let change=private_output(address,100_000_000-value,None)?;
            let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),Amount::from(100_000_000u64),[&funding,&change],Amount::zero())?;
            let nonce=RistrettoPublicKey::convert_from_byte_type(&shield.stealth_outputs()[0].output.sender_public_nonce).map_err(|e|anyhow::anyhow!(e.to_string()))?;
            let input_key=kdfs::owner_stealth_dh_secret(Network::Esmeralda,signers[i].credentials().account_secret(),&nonce);
            let outputs=allocations[i].iter().map(|v|private_output(address,*v,Some(component))).collect::<Result<Vec<_>>>()?;
            let cells=outputs.iter().map(|o|MaskAndValue{mask:o.witness.mask.clone(),value:o.witness.amount}).collect();
            let input=decrypt_output(signers[i].credentials(),&shield.stealth_outputs()[0])?;
            let pledge=private_transfer(vec![StealthInputWitness::new(input)],&outputs)?;
            let d=Deposit{shield,pledge,cells,input_key:hex::encode(input_key.as_bytes())};save_private(&path,&d)?;d
        };
        let portable=RecoveryPackage {
            version:1, network:"esmeralda".into(), indexer:INDEXER.into(), template:template.to_string(),
            component:component.to_string(), terms_cbor:hex::encode(tari_bor::encode(&terms)?), pledge_id:i as u32,
            account_secret:hex::encode(signers[i].credentials().account_secret().as_bytes()),
            view_secret:hex::encode(signers[i].credentials().view_only_secret().as_bytes()),
            shield_txid:recorded_txid(&format!("shield-{name}")), pledge_txid:recorded_txid(&format!("pledge-{name}")),
            deposit: Deposit { shield:deposit.shield.clone(),pledge:deposit.pledge.clone(),cells:deposit.cells.clone(),input_key:deposit.input_key.clone() },
        };
        save_private(&format!("portable-{name}"),&portable)?;
        let provider=&mut providers[i];
        let b=base(provider).await?.call_method(signers[i].address().to_account_address(),"withdraw",args![TARI_TOKEN,100_000_000u64])
            .put_last_instruction_output_on_workspace("shield-funds").stealth_transfer_with_input_bucket(TARI_TOKEN,deposit.shield.clone(),"shield-funds");
        execute(provider,&signers[i],b,&[],&format!("shield-{name}")).await?;
        let b=with_stealth_inputs(base(provider).await?.call_method(component,"pledge",args![i==3,deposit.pledge.clone()]),&deposit.pledge);
        let input_key=RistrettoSecretKey::from_canonical_bytes(&hex::decode(&deposit.input_key)?).map_err(|e|anyhow::anyhow!(e.to_string()))?;
        execute(provider,&signers[i],b,&[input_key],&format!("pledge-{name}")).await?;
        let mut portable=portable;
        portable.shield_txid=recorded_txid(&format!("shield-{name}"));
        portable.pledge_txid=recorded_txid(&format!("pledge-{name}"));
        save_private(&format!("portable-{name}"),&portable)?;
        deposits.push(deposit);
    }
    let masks=(0..3).flat_map(|j| [
        check_aggregate(&deposits[..3].iter().map(|d|d.cells[j].clone()).collect::<Vec<_>>(),terms.stages[j].community),
        check_aggregate(&[deposits[3].cells[j].clone()],terms.stages[j].sponsor)
    ]).collect::<Result<Vec<_>>>()?;
    if !done("activation") {
        rejects(p,operator,base(p).await?.call_method(component,"activate",args![vec![0u32,1,3],masks.clone()]),"below-threshold","exact confidential budget not met").await?;
        rejects(p,operator,base(p).await?.call_method(component,"activate",args![vec![3u32],masks.clone()]),"sponsor-as-community","missing funding role").await?;
        rejects(p,operator,base(p).await?.call_method(component,"activate",args![vec![0u32,1,2,3],masks[..2].to_vec()]),"missing-bundle-leg","all role/stage openings required").await?;
        rejects(p,operator,base(p).await?.call_method(component,"activate",args![vec![0u32,1,2,3,3],masks.clone()]),"duplicate-pledge","duplicate pledge").await?;
    }
    let b=base(p).await?.call_method(component,"activate",args![vec![0u32,1,2,3],masks.clone()]);
    execute(p,operator,b,&[],"activation").await?;
    rejects(p,operator,base(p).await?.call_method(component,"activate",args![vec![0u32,1,2,3],masks]),"duplicate-activation","activation closed").await?;
    let evidence=Hash32::from_array([42;32]);
    for j in 0..2 {
        let label=format!("release-{j}");
        if done(&label) {continue;}
        let inputs=deposits.iter().map(|d|escrow_input(d.cells[j].clone(),component)).collect::<Result<Vec<_>>>()?;
        let output=private_output(operator.address(),terms.stages[j].community+terms.stages[j].sponsor,None)?;
        let transfer=private_transfer(inputs,&[output])?;
        save_private(&format!("payout-{j}"),&transfer)?;
        if j==0 {
            rejects(p,operator,with_stealth_inputs(base(p).await?.stealth_transfer(TARI_TOKEN,transfer.clone()),&transfer),"direct-spend-bypass","No active call frame").await?;
            let mut extra=transfer.clone();extra.outputs_statement.outputs.push(extra.outputs_statement.outputs[0].clone());
            rejects(p,operator,with_stealth_inputs(base(p).await?.call_method(component,"release",args![0u32,evidence,extra.clone()]),&extra),"extra-output","one private output required").await?;
            let mut public=transfer.clone();public.outputs_statement.revealed_output_amount=Amount::from(1u64);
            rejects(p,operator,with_stealth_inputs(base(p).await?.call_method(component,"release",args![0u32,evidence,public.clone()]),&public),"public-outflow","public flow forbidden").await?;
        } else {
            rejects(p,operator,with_stealth_inputs(base(p).await?.call_method(component,"release",args![j as u32,evidence,transfer.clone()]),&transfer),"unapproved-release","review quorum not met").await?;
            for reviewer in 0..2 {
                let b=base(&providers[reviewer]).await?.call_method(component,"approve",args![j as u32,evidence]);
                execute(&mut providers[reviewer],&signers[reviewer],b,&[],&format!("approval-{j}-{reviewer}")).await?;
            }
        }
        let b=with_stealth_inputs(base(p).await?.call_method(component,"release",args![j as u32,evidence,transfer.clone()]),&transfer);
        execute(p,operator,b,&[],&label).await?;
        let actual=decrypt_output(operator.credentials(),&transfer.stealth_outputs()[0])?;
        ensure!(actual.value==terms.stages[j].community+terms.stages[j].sponsor,"payout decrypt mismatch");
        println!("Payout {j} decrypted; retains a valid one-time spending key.");
    }
    if !done("recipient-respend") {
        let paid:tari_template_lib::prelude::StealthTransferStatement=read_private("payout-0")?;
        let output=&paid.stealth_outputs()[0];
        let clear=decrypt_output(operator.credentials(),output)?;
        ensure!(clear.value==20_000_000,"recipient payout amount mismatch");
        let nonce=RistrettoPublicKey::convert_from_byte_type(&output.output.sender_public_nonce).map_err(|e|anyhow::anyhow!(e.to_string()))?;
        let key=kdfs::owner_stealth_dh_secret(Network::Esmeralda,operator.credentials().account_secret(),&nonce);
        let next=private_output(operator.address(),clear.value,None)?;
        let transfer=private_transfer(vec![StealthInputWitness::new(clear)],&[next])?;
        save_private("recipient-respend",&transfer)?;
        let b=with_stealth_inputs(base(p).await?.stealth_transfer(TARI_TOKEN,transfer.clone()),&transfer);
        execute(p,operator,b,&[key],"recipient-respend").await?;
        ensure!(decrypt_output(operator.credentials(),&transfer.stealth_outputs()[0])?.value==20_000_000,"respent output decrypt mismatch");
        println!("Recipient decrypted and independently spent the first payout.");
    }
    println!("Bundle activated and two stages paid. Audit remains refundable at epoch {}. Recovery requires no coordinator.",terms.stages[2].deadline);
    Ok(())
}

/// One funded community pledge with no sponsor, left to expire on chain.
pub async fn run_missed(p: &mut IndexerProvider<Wallet>, operator: &PrivateKeyProvider) -> Result<()> {
    let template=TemplateAddress::from_str(
        &receipt("publish")?.diff_summary.upped.iter()
            .find_map(|s|s.substate_id.to_string().strip_prefix("template_").map(str::to_owned))
            .ok_or_else(||anyhow::anyhow!("published template missing"))?)?;
    let donor=identity_named("missed")?;
    let sponsor=identity_named("sponsor")?;
    let mut dp=ProviderBuilder::new().wallet(OotleWallet::from(donor.clone())).connect(INDEXER).await?;
    if !done("faucet-missed") {
        let unsigned=IFaucet::new(&dp,Epoch(dp.get_epoch().await?.as_u64()+10)).take_faucet_funds().pay_fee(1_000_000u64).prepare().await?;
        submit(&mut dp,unsigned,"faucet-missed").await?;
    }
    let terms:Terms=if Path::new(".local/missed-terms.recovery").exists() {
        let bytes:String=read_private("missed-terms")?;tari_bor::decode(&hex::decode(bytes)?)?
    } else {
        let epoch=p.get_epoch().await?.as_u64();
        let terms=Terms{version:1,nonce:Hash32::from_array(rand::random()),resource:TARI_TOKEN,
            funding_deadline:epoch+3,sponsor_key:*sponsor.address().account_public_key(),
            stages:vec![Stage{leg:0,community:3_000_000,sponsor:2_000_000,
                recipient:*operator.address().account_public_key(),reviewers:vec![],quorum:0,
                deadline:epoch+4,upfront:true}]};
        save_private("missed-terms",&hex::encode(tari_bor::encode(&terms)?))?;terms
    };
    let b=base(p).await?.call_function(template,"new",args![terms.clone()]);
    let created=execute(p,operator,b,&[],"missed-campaign").await?;
    let component=created.diff_summary.upped.iter()
        .find_map(|s|if s.version==0{s.substate_id.as_component_address()}else{None})
        .ok_or_else(||anyhow::anyhow!("missed component missing"))?;
    fs::write("evidence/missed-deployment.json",serde_json::to_vec_pretty(&json!({
        "network":"esmeralda","component":component.to_string(),"template":template.to_string(),
        "funding_deadline":terms.funding_deadline,"community_budget_microtari":3_000_000,
        "sponsor_budget_microtari":2_000_000,"sponsor_funded":false}))?)?;
    let deposit:Deposit=if Path::new(".local/missed-deposit.recovery").exists(){read_private("missed-deposit")?}else{
        let funding=private_output(donor.address(),3_000_000,None)?;
        let change=private_output(donor.address(),97_000_000,None)?;
        let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),Amount::from(100_000_000u64),[&funding,&change],Amount::zero())?;
        let nonce=RistrettoPublicKey::convert_from_byte_type(&shield.stealth_outputs()[0].output.sender_public_nonce).map_err(|e|anyhow::anyhow!(e.to_string()))?;
        let input_key=kdfs::owner_stealth_dh_secret(Network::Esmeralda,donor.credentials().account_secret(),&nonce);
        let output=private_output(donor.address(),3_000_000,Some(component))?;
        let cell=MaskAndValue{mask:output.witness.mask.clone(),value:output.witness.amount};
        let input=decrypt_output(donor.credentials(),&shield.stealth_outputs()[0])?;
        let pledge=private_transfer(vec![StealthInputWitness::new(input)],&[output])?;
        let deposit=Deposit{shield,pledge,cells:vec![cell],input_key:hex::encode(input_key.as_bytes())};
        save_private("missed-deposit",&deposit)?;deposit
    };
    let portable=RecoveryPackage{version:1,network:"esmeralda".into(),indexer:INDEXER.into(),
        template:template.to_string(),component:component.to_string(),terms_cbor:hex::encode(tari_bor::encode(&terms)?),
        pledge_id:0,account_secret:hex::encode(donor.credentials().account_secret().as_bytes()),
        view_secret:hex::encode(donor.credentials().view_only_secret().as_bytes()),
        shield_txid:recorded_txid("missed-shield"),pledge_txid:recorded_txid("missed-pledge"),
        deposit:Deposit{shield:deposit.shield.clone(),pledge:deposit.pledge.clone(),cells:deposit.cells.clone(),input_key:deposit.input_key.clone()}};
    save_private("portable-missed",&portable)?;
    let b=base(&dp).await?.call_method(donor.address().to_account_address(),"withdraw",args![TARI_TOKEN,100_000_000u64])
        .put_last_instruction_output_on_workspace("missed-funds")
        .stealth_transfer_with_input_bucket(TARI_TOKEN,deposit.shield.clone(),"missed-funds");
    execute(&mut dp,&donor,b,&[],"missed-shield").await?;
    let key=RistrettoSecretKey::from_canonical_bytes(&hex::decode(&deposit.input_key)?).map_err(|e|anyhow::anyhow!(e.to_string()))?;
    let b=with_stealth_inputs(base(&dp).await?.call_method(component,"pledge",args![false,deposit.pledge.clone()]),&deposit.pledge);
    execute(&mut dp,&donor,b,&[key],"missed-pledge").await?;
    let mut portable=portable;
    portable.shield_txid=recorded_txid("missed-shield");portable.pledge_txid=recorded_txid("missed-pledge");
    save_private("portable-missed",&portable)?;
    let community=check_aggregate(&deposit.cells,3_000_000)?;
    rejects(p,operator,base(p).await?.call_method(component,"activate",args![vec![0u32],vec![community,Scalar32Bytes::from([0u8;32])]]),
        "missed-incomplete-bundle","missing funding role").await?;
    println!("Incomplete bundle rejected. Owner refund opens at epoch {} using portable-missed.recovery.",terms.funding_deadline);
    Ok(())
}

/// A short-deadline campaign that pays an upfront tranche and deliberately leaves
/// the second tranche untouched for an independent partial-delivery recovery.
pub async fn run_partial(p: &mut IndexerProvider<Wallet>, operator: &PrivateKeyProvider) -> Result<()> {
    let template=TemplateAddress::from_str(
        &receipt("publish")?.diff_summary.upped.iter()
            .find_map(|s|s.substate_id.to_string().strip_prefix("template_").map(str::to_owned))
            .ok_or_else(||anyhow::anyhow!("published template missing"))?)?;
    let names=["partial-community","partial-sponsor"];
    let mut signers=Vec::new();
    let mut providers=Vec::new();
    for name in &names {
        let signer=identity_named(name)?;
        let mut provider=ProviderBuilder::new().wallet(OotleWallet::from(signer.clone())).connect(INDEXER).await?;
        if !done(&format!("faucet-{name}")) {
            let unsigned=IFaucet::new(&provider,Epoch(provider.get_epoch().await?.as_u64()+10)).take_faucet_funds().pay_fee(1_000_000u64).prepare().await?;
            submit(&mut provider,unsigned,&format!("faucet-{name}")).await?;
        }
        signers.push(signer);
        providers.push(provider);
    }
    let reviewer=identity_named("alice")?;
    let terms:Terms=if Path::new(".local/partial-terms.recovery").exists() {
        let bytes:String=read_private("partial-terms")?;
        tari_bor::decode(&hex::decode(bytes)?)?
    } else {
        let epoch=p.get_epoch().await?.as_u64();
        let recipient=*operator.address().account_public_key();
        let terms=Terms{version:1,nonce:Hash32::from_array(rand::random()),resource:TARI_TOKEN,
            funding_deadline:epoch+2,sponsor_key:*signers[1].address().account_public_key(),stages:vec![
                Stage{leg:0,community:3_000_000,sponsor:2_000_000,recipient,
                    reviewers:vec![],quorum:0,deadline:epoch+3,upfront:true},
                Stage{leg:1,community:3_000_000,sponsor:2_000_000,recipient,
                    reviewers:vec![*reviewer.address().account_public_key()],quorum:1,deadline:epoch+4,upfront:false},
            ]};
        save_private("partial-terms",&hex::encode(tari_bor::encode(&terms)?))?;
        terms
    };
    let created=execute(p,operator,base(p).await?.call_function(template,"new",args![terms.clone()]),&[],"partial-campaign").await?;
    let component=created.diff_summary.upped.iter()
        .find_map(|s|if s.version==0{s.substate_id.as_component_address()}else{None})
        .ok_or_else(||anyhow::anyhow!("partial component missing"))?;
    fs::write("evidence/partial-deployment.json",serde_json::to_vec_pretty(&json!({
        "network":"esmeralda","indexer":INDEXER,"template":template.to_string(),
        "component":component.to_string(),"funding_deadline":terms.funding_deadline,
        "upfront_deadline":terms.stages[0].deadline,"partial_refund_epoch":terms.stages[1].deadline,
        "community_budget_microtari":6_000_000,"sponsor_budget_microtari":4_000_000,
        "upfront_payment_microtari":5_000_000,"remaining_refund_microtari":5_000_000
    }))?)?;
    let allocations=[[3_000_000,3_000_000],[2_000_000,2_000_000]];
    let mut deposits=Vec::new();
    for i in 0..2 {
        let name=names[i];
        let label=format!("partial-deposit-{name}");
        let deposit:Deposit=if Path::new(&format!(".local/{label}.recovery")).exists() {
            read_private(&label)?
        } else {
            let address=signers[i].address();
            let total=allocations[i].iter().sum::<u64>();
            let funding=private_output(address,total,None)?;
            let change=private_output(address,100_000_000-total,None)?;
            let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),Amount::from(100_000_000u64),[&funding,&change],Amount::zero())?;
            let nonce=RistrettoPublicKey::convert_from_byte_type(&shield.stealth_outputs()[0].output.sender_public_nonce).map_err(|e|anyhow::anyhow!(e.to_string()))?;
            let input_key=kdfs::owner_stealth_dh_secret(Network::Esmeralda,signers[i].credentials().account_secret(),&nonce);
            let outputs=allocations[i].iter().map(|v|private_output(address,*v,Some(component))).collect::<Result<Vec<_>>>()?;
            let cells=outputs.iter().map(|o|MaskAndValue{mask:o.witness.mask.clone(),value:o.witness.amount}).collect();
            let input=decrypt_output(signers[i].credentials(),&shield.stealth_outputs()[0])?;
            let pledge=private_transfer(vec![StealthInputWitness::new(input)],&outputs)?;
            let deposit=Deposit{shield,pledge,cells,input_key:hex::encode(input_key.as_bytes())};
            save_private(&label,&deposit)?;
            deposit
        };
        let shield_label=format!("partial-shield-{name}");
        let pledge_label=format!("partial-pledge-{name}");
        let portable_label=format!("partial-portable-{name}");
        let mut portable=RecoveryPackage{version:1,network:"esmeralda".into(),indexer:INDEXER.into(),
            template:template.to_string(),component:component.to_string(),terms_cbor:hex::encode(tari_bor::encode(&terms)?),
            pledge_id:i as u32,account_secret:hex::encode(signers[i].credentials().account_secret().as_bytes()),
            view_secret:hex::encode(signers[i].credentials().view_only_secret().as_bytes()),
            shield_txid:recorded_txid(&shield_label),pledge_txid:recorded_txid(&pledge_label),
            deposit:Deposit{shield:deposit.shield.clone(),pledge:deposit.pledge.clone(),cells:deposit.cells.clone(),input_key:deposit.input_key.clone()}};
        save_private(&portable_label,&portable)?;
        let provider=&mut providers[i];
        let b=base(provider).await?.call_method(signers[i].address().to_account_address(),"withdraw",args![TARI_TOKEN,100_000_000u64])
            .put_last_instruction_output_on_workspace("partial-funds")
            .stealth_transfer_with_input_bucket(TARI_TOKEN,deposit.shield.clone(),"partial-funds");
        execute(provider,&signers[i],b,&[],&shield_label).await?;
        let key=RistrettoSecretKey::from_canonical_bytes(&hex::decode(&deposit.input_key)?).map_err(|e|anyhow::anyhow!(e.to_string()))?;
        let b=with_stealth_inputs(base(provider).await?.call_method(component,"pledge",args![i==1,deposit.pledge.clone()]),&deposit.pledge);
        execute(provider,&signers[i],b,&[key],&pledge_label).await?;
        portable.shield_txid=recorded_txid(&shield_label);
        portable.pledge_txid=recorded_txid(&pledge_label);
        save_private(&portable_label,&portable)?;
        deposits.push(deposit);
    }
    let masks=(0..2).flat_map(|j|[
        check_aggregate(&[deposits[0].cells[j].clone()],terms.stages[j].community),
        check_aggregate(&[deposits[1].cells[j].clone()],terms.stages[j].sponsor)
    ]).collect::<Result<Vec<_>>>()?;
    execute(p,operator,base(p).await?.call_method(component,"activate",args![vec![0u32,1u32],masks]),&[],"partial-activation").await?;
    if !done("partial-release-0") {
        let inputs=deposits.iter().map(|d|escrow_input(d.cells[0].clone(),component)).collect::<Result<Vec<_>>>()?;
        let output=private_output(operator.address(),5_000_000,None)?;
        let transfer=private_transfer(inputs,&[output])?;
        save_private("partial-payout-0",&transfer)?;
        let b=with_stealth_inputs(base(p).await?.call_method(component,"release",args![0u32,Hash32::from_array([61;32]),transfer.clone()]),&transfer);
        execute(p,operator,b,&[],"partial-release-0").await?;
        ensure!(decrypt_output(operator.credentials(),&transfer.stealth_outputs()[0])?.value==5_000_000,"upfront payout decryption mismatch");
    }
    println!("Partial campaign paid its upfront tranche; separate community and sponsor refunds open at epoch {}.",terms.stages[1].deadline);
    Ok(())
}

/// Capacity fixture: N-1 community pledge records and one sponsor pledge,
/// followed by an N-input private payout. Community records reuse a few test
/// keys; this measures transaction capacity, not unique people.
pub async fn run_scale(p: &mut IndexerProvider<Wallet>, operator: &PrivateKeyProvider, pledge_count:u32) -> Result<()> {
    ensure!([8,16,32].contains(&pledge_count),"supported live capacity fixtures are 8, 16 and 32 pledges");
    let prefix=format!("scale{pledge_count}");
    let community_count=pledge_count-1;
    let community_amount=u64::from(community_count)*1_000_000;
    let total_amount=u64::from(pledge_count)*1_000_000;
    let template=TemplateAddress::from_str(
        &receipt("publish")?.diff_summary.upped.iter()
            .find_map(|s|s.substate_id.to_string().strip_prefix("template_").map(str::to_owned))
            .ok_or_else(||anyhow::anyhow!("published template missing"))?)?;
    let community_batches=if pledge_count==32 {vec![15,15,1]} else {vec![community_count]};
    let mut names=community_batches.iter().enumerate().map(|(i,_)|
        if community_batches.len()==1 {format!("{prefix}-community")} else {format!("{prefix}-community-{i}")}
    ).collect::<Vec<_>>();
    names.push(format!("{prefix}-sponsor"));
    let sponsor_role=names.len()-1;
    let mut signers=Vec::new();
    let mut providers=Vec::new();
    for name in &names {
        let signer=identity_named(name)?;
        let mut provider=ProviderBuilder::new().wallet(OotleWallet::from(signer.clone())).connect(INDEXER).await?;
        if !done(&format!("faucet-{name}")) {
            let unsigned=IFaucet::new(&provider,Epoch(provider.get_epoch().await?.as_u64()+10)).take_faucet_funds().pay_fee(1_000_000u64).prepare().await?;
            submit(&mut provider,unsigned,&format!("faucet-{name}")).await?;
        }
        signers.push(signer);
        providers.push(provider);
    }
    let terms_label=format!("{prefix}-terms");
    let terms:Terms=if Path::new(&format!(".local/{terms_label}.recovery")).exists() {
        let bytes:String=read_private(&terms_label)?;
        tari_bor::decode(&hex::decode(bytes)?)?
    } else {
        let epoch=p.get_epoch().await?.as_u64();
        let terms=Terms{version:1,nonce:Hash32::from_array(rand::random()),resource:TARI_TOKEN,
            funding_deadline:epoch+20,sponsor_key:*signers[sponsor_role].address().account_public_key(),
            stages:vec![Stage{leg:0,community:community_amount,sponsor:1_000_000,
                recipient:*operator.address().account_public_key(),reviewers:vec![],quorum:0,
                deadline:epoch+21,upfront:true}]};
        save_private(&terms_label,&hex::encode(tari_bor::encode(&terms)?))?;
        terms
    };
    let created=execute(p,operator,base(p).await?.call_function(template,"new",args![terms.clone()]),&[],&format!("{prefix}-campaign")).await?;
    let component=created.diff_summary.upped.iter()
        .find_map(|s|if s.version==0{s.substate_id.as_component_address()}else{None})
        .ok_or_else(||anyhow::anyhow!("capacity fixture component missing"))?;
    fs::write(format!("evidence/{prefix}-deployment.json"),serde_json::to_vec_pretty(&json!({
        "network":"esmeralda","indexer":INDEXER,"template":template.to_string(),
        "component":component.to_string(),"funding_deadline":terms.funding_deadline,
        "community_pledge_records":community_count,"sponsor_pledge_records":1,
        "distinct_community_signing_keys":community_batches.len(),"total_budget_microtari":total_amount
    }))?)?;
    let mut deposits=Vec::new();
    for (role,name) in names.iter().enumerate() {
        let count=if role==sponsor_role {1} else {community_batches[role]};
        let label=format!("{prefix}-{name}-deposits");
        let role_deposits:Vec<Deposit>=if Path::new(&format!(".local/{label}.recovery")).exists() {
            read_private(&label)?
        } else {
            let address=signers[role].address();
            let mut outputs=(0..count).map(|_|private_output(address,1_000_000,None)).collect::<Result<Vec<_>>>()?;
            outputs.push(private_output(address,100_000_000-u64::from(count)*1_000_000,None)?);
            let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),
                Amount::from(100_000_000u64),outputs.iter(),Amount::zero())?;
            let mut role_deposits=Vec::new();
            for i in 0..count as usize {
                let output=&shield.stealth_outputs()[i];
                let nonce=RistrettoPublicKey::convert_from_byte_type(&output.output.sender_public_nonce)
                    .map_err(|e|anyhow::anyhow!(e.to_string()))?;
                let input_key=kdfs::owner_stealth_dh_secret(Network::Esmeralda,signers[role].credentials().account_secret(),&nonce);
                let clear=decrypt_output(signers[role].credentials(),output)?;
                ensure!(clear.value==1_000_000,"capacity fixture shield output amount mismatch");
                let locked=private_output(address,1_000_000,Some(component))?;
                let cell=MaskAndValue{mask:locked.witness.mask.clone(),value:locked.witness.amount};
                let pledge=private_transfer(vec![StealthInputWitness::new(clear)],&[locked])?;
                role_deposits.push(Deposit{shield:shield.clone(),pledge,cells:vec![cell],
                    input_key:hex::encode(input_key.as_bytes())});
            }
            save_private(&label,&role_deposits)?;
            role_deposits
        };
        ensure!(role_deposits.len()==count as usize,"capacity fixture cached deposit count mismatch");
        let shield_label=format!("{prefix}-shield-{name}");
        let provider=&mut providers[role];
        let b=base(provider).await?.call_method(signers[role].address().to_account_address(),"withdraw",args![TARI_TOKEN,100_000_000u64])
            .put_last_instruction_output_on_workspace("capacity-funds")
            .stealth_transfer_with_input_bucket(TARI_TOKEN,role_deposits[0].shield.clone(),"capacity-funds");
        execute(provider,&signers[role],b,&[],&shield_label).await?;
        for deposit in &role_deposits {
            let id=deposits.len() as u32;
            let pledge_label=format!("{prefix}-pledge-{id}");
            let key=RistrettoSecretKey::from_canonical_bytes(&hex::decode(&deposit.input_key)?)
                .map_err(|e|anyhow::anyhow!(e.to_string()))?;
            let b=with_stealth_inputs(base(provider).await?.call_method(component,"pledge",args![role==sponsor_role,deposit.pledge.clone()]),&deposit.pledge);
            execute(provider,&signers[role],b,&[key],&pledge_label).await?;
            let portable=RecoveryPackage{version:1,network:"esmeralda".into(),indexer:INDEXER.into(),
                template:template.to_string(),component:component.to_string(),terms_cbor:hex::encode(tari_bor::encode(&terms)?),
                pledge_id:id,account_secret:hex::encode(signers[role].credentials().account_secret().as_bytes()),
                view_secret:hex::encode(signers[role].credentials().view_only_secret().as_bytes()),
                shield_txid:recorded_txid(&shield_label),pledge_txid:recorded_txid(&pledge_label),
                deposit:Deposit{shield:deposit.shield.clone(),pledge:deposit.pledge.clone(),
                    cells:deposit.cells.clone(),input_key:deposit.input_key.clone()}};
            save_private(&format!("{prefix}-portable-{id}"),&portable)?;
            deposits.push(deposit.clone());
        }
    }
    ensure!(deposits.len()==pledge_count as usize,"capacity fixture pledge count mismatch");
    let masks=vec![
        check_aggregate(&deposits[..community_count as usize].iter().map(|d|d.cells[0].clone()).collect::<Vec<_>>(),community_amount)?,
        check_aggregate(&[deposits[community_count as usize].cells[0].clone()],1_000_000)?,
    ];
    execute(p,operator,base(p).await?.call_method(component,"activate",args![(0u32..pledge_count).collect::<Vec<_>>(),masks]),&[],&format!("{prefix}-activation")).await?;
    let payout_label=format!("{prefix}-payout");
    let transfer:tari_template_lib::prelude::StealthTransferStatement=if Path::new(&format!(".local/{payout_label}.recovery")).exists() {
        read_private(&payout_label)?
    } else {
        let inputs=deposits.iter().map(|d|escrow_input(d.cells[0].clone(),component)).collect::<Result<Vec<_>>>()?;
        let output=private_output(operator.address(),total_amount,None)?;
        let transfer=private_transfer(inputs,&[output])?;
        save_private(&payout_label,&transfer)?;
        transfer
    };
    let b=with_stealth_inputs(base(p).await?.call_method(component,"release",args![0u32,Hash32::from_array([81;32]),transfer.clone()]),&transfer);
    execute(p,operator,b,&[],&format!("{prefix}-release")).await?;
    ensure!(decrypt_output(operator.credentials(),&transfer.stealth_outputs()[0])?.value==total_amount,"capacity fixture payout decrypt mismatch");
    println!("{pledge_count} private pledge records activated and paid together on Esmeralda; {community_count} community records use {} signing keys.",community_batches.len());
    Ok(())
}

/// Dry-run both sides of the delivery/refund boundary on the live component.
/// Run this after the frozen epoch and before spending either remaining cell.
pub async fn run_partial_late_check(p: &IndexerProvider<Wallet>, operator: &PrivateKeyProvider) -> Result<()> {
    let deployment:serde_json::Value=serde_json::from_slice(&fs::read("evidence/partial-deployment.json")?)?;
    let component=ComponentAddress::from_str(deployment["component"].as_str().ok_or_else(||anyhow::anyhow!("missing partial component"))?)?;
    let deadline=deployment["partial_refund_epoch"].as_u64().ok_or_else(||anyhow::anyhow!("missing partial refund epoch"))?;
    let now=p.get_epoch().await?.as_u64();
    ensure!(now>=deadline,"partial refund not yet open: current epoch {now}, deadline {deadline}");
    let community:Deposit=read_private("partial-deposit-partial-community")?;
    let sponsor:Deposit=read_private("partial-deposit-partial-sponsor")?;
    let inputs=vec![escrow_input(community.cells[1].clone(),component)?,escrow_input(sponsor.cells[1].clone(),component)?];
    let output=private_output(operator.address(),5_000_000,None)?;
    let transfer=private_transfer(inputs,&[output])?;
    let evidence=Hash32::from_array([62;32]);
    let b=with_stealth_inputs(base(p).await?.call_method(component,"release",args![1u32,evidence,transfer.clone()]),&transfer);
    rejects(p,operator,b,"partial-late-release","delivery deadline passed").await?;
    let reviewer=identity_named("alice")?;
    let rp=ProviderBuilder::new().wallet(OotleWallet::from(reviewer.clone())).connect(INDEXER).await?;
    rejects(&rp,&reviewer,base(&rp).await?.call_method(component,"approve",args![1u32,evidence]),
        "partial-late-approval","delivery deadline passed").await?;
    println!("At epoch {now}, late release and approval both rejected; owner refunds may now proceed.");
    Ok(())
}

/// Commit a deliberately failing main intent with a valid fee intent, proving
/// that paying a fee is never mistaken for funding or activation.
pub async fn run_fee_only_check(p: &mut IndexerProvider<Wallet>, operator: &PrivateKeyProvider, direct: bool) -> Result<()> {
    let label=if direct {"direct-bypass-check"} else {"fee-only-check"};
    if done(label) {
        let r=recorded_receipt(label)?;
        ensure!(r.outcome.is_fee_intent_commit(),"saved receipt is not fee-only");
        println!("Previously confirmed fee-only outcome; no duplicate submitted.");
        return Ok(());
    }
    let deployment:serde_json::Value=serde_json::from_slice(&fs::read("evidence/deployment.json")?)?;
    let component=ComponentAddress::from_str(deployment["component"].as_str().ok_or_else(||anyhow::anyhow!("missing component"))?)?;
    let pending_path=format!("evidence/{label}-pending.json");
    if Path::new(&pending_path).exists() {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(&pending_path)?)?;
        let id=tari_ootle_transaction::TransactionId::from_hex(value["transaction_id"].as_str().ok_or_else(||anyhow::anyhow!("bad pending fee-only journal"))?)?;
        let client=p.weak_client().upgrade().ok_or_else(||anyhow::anyhow!("indexer connection lost"))?;
        let indexed=client.get_transaction_receipt(id.into_receipt_address()).await?;
        ensure!(indexed.receipt.outcome.is_fee_intent_commit(),"fee-only transaction had unexpected outcome");
        fs::write(format!("evidence/{label}-receipt.json"),serde_json::to_vec_pretty(&json!({"transaction_id":id.to_string(),"receipt":indexed.receipt}))?)?;
        println!("Fee-only transaction already recorded: {id}");
        return Ok(());
    }
    let builder=if direct {
        let deposits=[read_private::<Deposit>("deposit-alice")?,read_private::<Deposit>("deposit-bob")?,
            read_private::<Deposit>("deposit-carol")?,read_private::<Deposit>("deposit-sponsor")?];
        let inputs=deposits.iter().map(|d|escrow_input(d.cells[2].clone(),component)).collect::<Result<Vec<_>>>()?;
        let output=private_output(operator.address(),40_000_000,None)?;
        let transfer=private_transfer(inputs,&[output])?;
        with_stealth_inputs(base(p).await?.stealth_transfer(TARI_TOKEN,transfer.clone()),&transfer)
    } else {
        base(p).await?.call_method(component,"activate",args![Vec::<u32>::new(),Vec::<Scalar32Bytes>::new()])
    };
    let unsigned=resolve(p,builder).await?;
    let dry=p.send_dry_run(sign(unsigned.clone().with_dry_run(true),operator,&[])).await?;
    fs::write(format!("evidence/{label}-dry-run.json"),serde_json::to_vec_pretty(&dry)?)?;
    let rejection=dry.finalize.any_reject().ok_or_else(||anyhow::anyhow!("expected main intent to reject"))?.to_string();
    let expected=if direct {"No active call frame"} else {"activation closed"};
    ensure!(rejection.contains(expected),"unexpected rejection: {rejection}");
    let tx=sign(unsigned,operator,&[]);
    let txid=tx.calculate_id();
    fs::write(&pending_path,serde_json::to_vec_pretty(&json!({"transaction_id":txid.to_string(),"status":"prepared-or-submitted"}))?)?;
    let pending=p.send_transaction(tx).await?;
    ensure!(pending.tx_id()==txid,"submitted fee-only ID mismatch");
    let outcome=pending.watch().await?;
    ensure!(outcome.is_only_fee_commit(),"expected fee-only commit, got {outcome}");
    let r=pending.get_receipt().await?;
    ensure!(r.outcome.is_fee_intent_commit(),"receipt did not confirm fee-only outcome");
    ensure!(!r.diff_summary.upped.iter().any(|up|up.substate_id==SubstateId::Component(component)),"rejected main intent unexpectedly changed component");
    fs::write(format!("evidence/{label}-receipt.json"),serde_json::to_vec_pretty(&json!({"transaction_id":txid.to_string(),"receipt":r}))?)?;
    println!("Fee paid but main intent rejected: {txid} {outcome}");
    Ok(())
}

pub async fn recover_command() -> Result<()> {
    use tari_template_lib::types::stealth::SpendAuthorization;
    let inspect=std::env::args().nth(1).as_deref()==Some("inspect");
    let mut args=std::env::args().skip(2);
    let package_path=std::path::PathBuf::from(args.next().ok_or_else(||anyhow::anyhow!("usage: threshold recover PACKAGE.recovery UNLOCK.secret [INDEXER_URL]"))?);
    let unlock_path=std::path::PathBuf::from(args.next().ok_or_else(||anyhow::anyhow!("missing unlock file"))?);
    let indexer_override=args.next();
    ensure!(args.next().is_none(),"too many arguments");
    let pass=fs::read(&unlock_path)?;
    let package:RecoveryPackage=serde_json::from_slice(&decrypt_with_password(&fs::read(&package_path)?,&pass)?)?;
    ensure!(package.version==1 && package.network=="esmeralda", "unsupported recovery context");
    let component=ComponentAddress::from_str(&package.component)?;
    let template=TemplateAddress::from_str(&package.template)?;
    let secret=OotleSecretKey::new(Network::Esmeralda,
        RistrettoSecretKey::from_canonical_bytes(&hex::decode(&package.account_secret)?).map_err(|e|anyhow::anyhow!(e.to_string()))?,
        RistrettoSecretKey::from_canonical_bytes(&hex::decode(&package.view_secret)?).map_err(|e|anyhow::anyhow!(e.to_string()))?);
    let signer=PrivateKeyProvider::new(secret);
    let url=indexer_override.as_deref().unwrap_or(&package.indexer);
    let mut provider=ProviderBuilder::new().wallet(OotleWallet::from(signer.clone())).connect(url).await?;
    ensure!(provider.get_network().await?==Network::Esmeralda,"indexer network mismatch");
    let client=provider.weak_client().upgrade().ok_or_else(||anyhow::anyhow!("indexer connection lost"))?;
    let response=client.get_substate(&SubstateId::Component(component),Default::default()).await?;
    ensure!(response.verified,"component state is not proof-verified by indexer");
    let chain_component=response.substate.as_component().ok_or_else(||anyhow::anyhow!("not a component"))?;
    ensure!(*chain_component.template_address()==template && matches!(chain_component.owner_rule(),SubstateOwnerRule::None),"template or ownership mismatch");
    let state:StateSnapshot=tari_bor::from_value(chain_component.state())?;
    ensure!(state.terms.resource==TARI_TOKEN,"unsupported recovery resource");
    ensure!(hex::encode(tari_bor::encode(&state.terms)?)==package.terms_cbor,"frozen terms mismatch");
    let journal=package_path.with_extension("journal.json");
    let final_receipt=package_path.with_extension("receipt.json");
    if journal.exists() {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(&journal)?)?;
        ensure!(value["component"]==package.component && value["pledge_id"]==package.pledge_id,"recovery journal context mismatch");
        let txid=tari_ootle_transaction::TransactionId::from_hex(value["transaction_id"].as_str().ok_or_else(||anyhow::anyhow!("bad recovery journal"))?)?;
        if final_receipt.exists() {
            let receipt:TransactionReceipt=serde_json::from_slice(&fs::read(&final_receipt)?)?;
            ensure!(receipt.outcome.is_commit(),"saved recovery receipt is not a full commit");
            let pledge=state.pledges.get(package.pledge_id as usize).ok_or_else(||anyhow::anyhow!("pledge not registered"))?;
            ensure!(pledge.refunded && pledge.owner==*signer.address().account_public_key(),"committed owner refund not recorded in indexed component");
            println!("Recovery already committed: {txid} (saved full receipt; indexer component verified)");
            return Ok(());
        }
        let r=client.get_transaction_receipt(txid.into_receipt_address()).await?;
        ensure!(r.receipt.outcome.is_commit(),"recovery transaction did not commit: {:?}",r.receipt.outcome);
        let refreshed=client.get_substate(&SubstateId::Component(component),Default::default()).await?;
        ensure!(refreshed.verified,"refreshed component state is not proof-verified by indexer");
        let refreshed_component=refreshed.substate.as_component().ok_or_else(||anyhow::anyhow!("refreshed state is not a component"))?;
        ensure!(*refreshed_component.template_address()==template && matches!(refreshed_component.owner_rule(),SubstateOwnerRule::None),"refreshed template or ownership mismatch");
        let refreshed_state:StateSnapshot=tari_bor::from_value(refreshed_component.state())?;
        ensure!(hex::encode(tari_bor::encode(&refreshed_state.terms)?)==package.terms_cbor,"refreshed frozen terms mismatch");
        let refreshed_pledge=refreshed_state.pledges.get(package.pledge_id as usize).ok_or_else(||anyhow::anyhow!("refreshed pledge not registered"))?;
        ensure!(refreshed_pledge.refunded && refreshed_pledge.owner==*signer.address().account_public_key(),"committed refund not yet reflected in indexed component");
        fs::write(final_receipt,serde_json::to_vec_pretty(&r.receipt)?)?;
        println!("Recovery already committed: {txid}");
        return Ok(());
    }
    let pledge=state.pledges.get(package.pledge_id as usize).ok_or_else(||anyhow::anyhow!("pledge not registered"))?;
    ensure!(pledge.owner==*signer.address().account_public_key() && !pledge.refunded,"refund owner or status mismatch");
    ensure!(pledge.sponsor==(pledge.owner==state.terms.sponsor_key),"sponsor role mismatch");
    let output_cells=package.deposit.pledge.stealth_outputs().iter().map(|o|o.output.commitment).collect::<Vec<_>>();
    ensure!(pledge.cells==output_cells && pledge.cells.len()==package.deposit.cells.len(),"pledge commitments mismatch");
    let root=escrow_root(component);
    ensure!(package.deposit.pledge.stealth_outputs().iter().all(|o|o.auth==SpendAuthorization::Script(root)),"escrow root mismatch");
    let selected=state.active && state.selected.contains(&package.pledge_id);
    let now=provider.get_epoch().await?.as_u64();
    let (start,opens_at)=if selected {
        let current=state.terms.stages.get(state.next_stage as usize).ok_or_else(||anyhow::anyhow!("all stages paid"))?;
        (state.next_stage as usize,current.deadline)
    } else {
        (0,if state.active {0} else {state.terms.funding_deadline})
    };
    let cells=&package.deposit.cells[start..];
    ensure!(!cells.is_empty(),"nothing eligible to refund");
    let amount=cells.iter().try_fold(0u64,|sum,cell|sum.checked_add(cell.value)).ok_or_else(||anyhow::anyhow!("refund amount overflow"))?;
    for (j,cell) in cells.iter().enumerate() {
        let commitment=pledge.cells[start+j];
        ensure!(cell.to_commitment().to_byte_type()==commitment,"opening mismatch");
        let address=UtxoAddress::new(TARI_TOKEN,commitment.into());
        let output=client.get_substate(&SubstateId::Utxo(address),Default::default()).await?;
        ensure!(output.verified,"eligible cell not proof-verified by indexer");
        let utxo=output.substate.as_utxo().ok_or_else(||anyhow::anyhow!("eligible cell not UTXO"))?;
        ensure!(utxo.output().ok_or_else(||anyhow::anyhow!("eligible cell already spent"))?.auth==SpendAuthorization::Script(root),"eligible cell lock mismatch");
    }
    println!("Pledge {}: {} remaining cells, {} microtari. Current epoch {}. Refund opens at epoch {}.",package.pledge_id,cells.len(),amount,now,opens_at);
    if inspect {return Ok(());}
    ensure!(now>=opens_at,"refund not yet eligible");
    let inputs=cells.iter().cloned().map(|c|escrow_input(c,component)).collect::<Result<Vec<_>>>()?;
    let output=private_output(signer.address(),amount,None)?;
    let proof_started=std::time::Instant::now();
    let transfer=private_transfer(inputs,&[output])?;
    let proof_ms=proof_started.elapsed().as_millis();
    let output_backup=package_path.with_extension("refund-output.recovery");
    fs::write(&output_backup,encrypt_with_password(&serde_json::to_vec(&transfer)?,&pass)?)?;
    let builder=with_stealth_inputs(base(&provider).await?.call_method(component,"refund",args![package.pledge_id,transfer.clone()]),&transfer);
    let unsigned=resolve(&provider,builder).await?;
    let dry=provider.send_dry_run(sign(unsigned.clone().with_dry_run(true),&signer,&[])).await?;
    fs::write(package_path.with_extension("dry-run.json"),serde_json::to_vec_pretty(&dry)?)?;
    ensure!(dry.finalize.any_reject().is_none(),"refund dry run rejected: {}",dry.finalize.result);
    let tx=sign(unsigned,&signer,&[]);
    let txid=tx.calculate_id();
    let tx_bytes=tari_bor::encode(&tx)?.len();
    fs::write(&journal,serde_json::to_vec_pretty(&json!({"transaction_id":txid.to_string(),"status":"prepared-or-submitted","component":component.to_string(),"pledge_id":package.pledge_id,"amount_microtari":amount,"native_proof_ms":proof_ms,"encoded_transaction_bytes":tx_bytes}))?)?;
    let pending=provider.send_transaction(tx).await?;
    let outcome=pending.watch().await?;
    ensure!(outcome.is_commit(),"refund main intent did not commit: {outcome}");
    let r=pending.get_receipt().await?;
    ensure!(r.outcome.is_commit(),"refund receipt not full commit");
    fs::write(final_receipt,serde_json::to_vec_pretty(&r)?)?;
    println!("Recovered {amount} microtari in {txid}. Private output backup: {}",output_backup.display());
    Ok(())
}

/// Verify the private refund against a fresh indexed UTXO, then optionally spend it.
/// This command works from the same portable directory as `recover`.
pub async fn verify_refund_command(respend: bool) -> Result<()> {
    let mut args=std::env::args().skip(2);
    let package_path=std::path::PathBuf::from(args.next().ok_or_else(||anyhow::anyhow!("missing recovery package"))?);
    let unlock_path=std::path::PathBuf::from(args.next().ok_or_else(||anyhow::anyhow!("missing unlock file"))?);
    let indexer_override=args.next();
    ensure!(args.next().is_none(),"too many arguments");
    let pass=fs::read(&unlock_path)?;
    let package:RecoveryPackage=serde_json::from_slice(&decrypt_with_password(&fs::read(&package_path)?,&pass)?)?;
    ensure!(package.version==1 && package.network=="esmeralda","unsupported recovery context");
    let component=ComponentAddress::from_str(&package.component)?;
    let template=TemplateAddress::from_str(&package.template)?;
    let secret=OotleSecretKey::new(Network::Esmeralda,
        RistrettoSecretKey::from_canonical_bytes(&hex::decode(&package.account_secret)?).map_err(|e|anyhow::anyhow!(e.to_string()))?,
        RistrettoSecretKey::from_canonical_bytes(&hex::decode(&package.view_secret)?).map_err(|e|anyhow::anyhow!(e.to_string()))?);
    let signer=PrivateKeyProvider::new(secret);
    let url=indexer_override.as_deref().unwrap_or(&package.indexer);
    let mut provider=ProviderBuilder::new().wallet(OotleWallet::from(signer.clone())).connect(url).await?;
    ensure!(provider.get_network().await?==Network::Esmeralda,"indexer network mismatch");
    let client=provider.weak_client().upgrade().ok_or_else(||anyhow::anyhow!("indexer connection lost"))?;
    let journal:serde_json::Value=serde_json::from_slice(&fs::read(package_path.with_extension("journal.json"))?)?;
    ensure!(journal["component"]==package.component && journal["pledge_id"]==package.pledge_id,"refund journal context mismatch");
    let amount=journal["amount_microtari"].as_u64().ok_or_else(||anyhow::anyhow!("refund amount missing"))?;
    let receipt:TransactionReceipt=serde_json::from_slice(&fs::read(package_path.with_extension("receipt.json"))?)?;
    ensure!(receipt.outcome.is_commit(),"refund receipt is not a full commit");
    let response=client.get_substate(&SubstateId::Component(component),Default::default()).await?;
    ensure!(response.verified,"component state is not proof-verified by indexer");
    let chain_component=response.substate.as_component().ok_or_else(||anyhow::anyhow!("not a component"))?;
    ensure!(*chain_component.template_address()==template && matches!(chain_component.owner_rule(),SubstateOwnerRule::None),"template or ownership mismatch");
    let state:StateSnapshot=tari_bor::from_value(chain_component.state())?;
    ensure!(hex::encode(tari_bor::encode(&state.terms)?)==package.terms_cbor,"frozen terms mismatch");
    let pledge=state.pledges.get(package.pledge_id as usize).ok_or_else(||anyhow::anyhow!("pledge not registered"))?;
    ensure!(pledge.refunded && pledge.owner==*signer.address().account_public_key(),"owner refund not recorded in component");
    let respend_receipt=package_path.with_extension("respend.receipt.json");
    let respend_journal=package_path.with_extension("respend.journal.json");
    if respend && respend_journal.exists() {
        if !respend_receipt.exists() {
            let record:serde_json::Value=serde_json::from_slice(&fs::read(&respend_journal)?)?;
            let id=tari_ootle_transaction::TransactionId::from_hex(record["transaction_id"].as_str().ok_or_else(||anyhow::anyhow!("bad re-spend journal"))?)?;
            let indexed=client.get_transaction_receipt(id.into_receipt_address()).await?;
            ensure!(indexed.receipt.outcome.is_commit(),"re-spend did not commit");
            fs::write(&respend_receipt,serde_json::to_vec_pretty(&indexed.receipt)?)?;
        }
    }
    let transfer:tari_template_lib::prelude::StealthTransferStatement=serde_json::from_slice(&decrypt_with_password(
        &fs::read(package_path.with_extension("refund-output.recovery"))?,&pass)?)?;
    ensure!(transfer.stealth_outputs().len()==1,"refund output count mismatch");
    let output=&transfer.stealth_outputs()[0];
    let address=UtxoAddress::new(TARI_TOKEN,output.output.commitment.into());
    ensure!(receipt.diff_summary.upped.iter().any(|up|up.substate_id==SubstateId::Utxo(address.clone())),"refund receipt does not create saved output");
    let clear=decrypt_output(signer.credentials(),output)?;
    ensure!(clear.value==amount && clear.to_commitment().to_byte_type()==output.output.commitment,"refund decryption mismatch");
    if respend_receipt.exists() {
        let next_receipt:TransactionReceipt=serde_json::from_slice(&fs::read(&respend_receipt)?)?;
        ensure!(next_receipt.outcome.is_commit(),"saved re-spend did not commit");
        let next_transfer:tari_template_lib::prelude::StealthTransferStatement=serde_json::from_slice(&decrypt_with_password(
            &fs::read(package_path.with_extension("respend-output.recovery"))?,&pass)?)?;
        ensure!(next_transfer.stealth_inputs().len()==1 && next_transfer.stealth_inputs()[0].commitment==output.output.commitment,"re-spend does not consume recovered output");
        ensure!(next_transfer.stealth_outputs().len()==1,"re-spend output count mismatch");
        let next=&next_transfer.stealth_outputs()[0];
        let next_address=UtxoAddress::new(TARI_TOKEN,next.output.commitment.into());
        ensure!(next_receipt.diff_summary.upped.iter().any(|up|up.substate_id==SubstateId::Utxo(next_address.clone())),"re-spend receipt does not create saved output");
        let next_clear=decrypt_output(signer.credentials(),next)?;
        ensure!(next_clear.value==amount && next_clear.to_commitment().to_byte_type()==next.output.commitment,"re-spend decryption mismatch");
        let next_response=client.get_substate(&SubstateId::Utxo(next_address),Default::default()).await?;
        ensure!(next_response.verified,"re-spend output is not proof-verified by indexer");
        let next_utxo=next_response.substate.as_utxo().ok_or_else(||anyhow::anyhow!("re-spend output not a UTXO"))?;
        let live=next_utxo.output().ok_or_else(||anyhow::anyhow!("re-spend output already spent"))?;
        ensure!(live.output.public_nonce==next.output.sender_public_nonce &&
            live.output.encrypted_data==next.output.encrypted_data && live.auth==next.auth,"re-spend output mismatch");
        println!("Refund output has been privately re-spent; new output verified and decrypted: {amount} microtari.");
        return Ok(());
    }
    let response=client.get_substate(&SubstateId::Utxo(address),Default::default()).await?;
    ensure!(response.verified,"refund output is not proof-verified by indexer");
    let utxo=response.substate.as_utxo().ok_or_else(||anyhow::anyhow!("refund output not a UTXO"))?;
    let chain_output=utxo.output().ok_or_else(||anyhow::anyhow!("refund output already spent without a saved re-spend receipt"))?;
    ensure!(chain_output.output.public_nonce==output.output.sender_public_nonce &&
        chain_output.output.encrypted_data==output.output.encrypted_data && chain_output.auth==output.auth,"refund output mismatch");
    println!("Refund output verified and decrypted: {amount} microtari. Fee: {} microtari.",receipt.fee_receipt.total_fees_paid());
    if !respend {return Ok(());}
    let nonce=RistrettoPublicKey::convert_from_byte_type(&output.output.sender_public_nonce).map_err(|e|anyhow::anyhow!(e.to_string()))?;
    let key=kdfs::owner_stealth_dh_secret(Network::Esmeralda,signer.credentials().account_secret(),&nonce);
    let next=private_output(signer.address(),amount,None)?;
    let spend=private_transfer(vec![StealthInputWitness::new(clear)],&[next])?;
    fs::write(package_path.with_extension("respend-output.recovery"),encrypt_with_password(&serde_json::to_vec(&spend)?,&pass)?)?;
    let builder=with_stealth_inputs(base(&provider).await?.stealth_transfer(TARI_TOKEN,spend.clone()),&spend);
    let unsigned=resolve(&provider,builder).await?;
    let dry=provider.send_dry_run(sign(unsigned.clone().with_dry_run(true),&signer,&[key.clone()])).await?;
    fs::write(package_path.with_extension("respend.dry-run.json"),serde_json::to_vec_pretty(&dry)?)?;
    ensure!(dry.finalize.any_reject().is_none(),"re-spend dry run rejected: {}",dry.finalize.result);
    let tx=sign(unsigned,&signer,&[key]);
    let txid=tx.calculate_id();
    fs::write(&respend_journal,serde_json::to_vec_pretty(&json!({"transaction_id":txid.to_string(),"status":"prepared-or-submitted","amount_microtari":amount,"encoded_transaction_bytes":tari_bor::encode(&tx)?.len()}))?)?;
    let pending=provider.send_transaction(tx).await?;
    ensure!(pending.tx_id()==txid,"submitted re-spend transaction ID mismatch");
    let outcome=pending.watch().await?;
    ensure!(outcome.is_commit(),"re-spend main intent did not commit: {outcome}");
    let receipt=pending.get_receipt().await?;
    ensure!(receipt.outcome.is_commit(),"re-spend receipt not full commit");
    fs::write(&respend_receipt,serde_json::to_vec_pretty(&receipt)?)?;
    ensure!(decrypt_output(signer.credentials(),&spend.stealth_outputs()[0])?.value==amount,"re-spend output decryption mismatch");
    println!("Recovered output independently re-spent in {txid}.");
    Ok(())
}
