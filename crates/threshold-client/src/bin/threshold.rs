use anyhow::{Result, ensure};
use std::{fs, path::Path};
use ootle_rs::{Network, ToAccountAddress, keys::OotleSecretKey, key_provider::PrivateKeyProvider,
    wallet::{OotleWallet, NetworkWallet}, provider::{Provider, ProviderBuilder, IndexerProvider, WalletProvider},
    builtin_templates::{UnsignedTransactionBuilder, faucet::IFaucet, account::IAccount}};
use tari_crypto::{keys::SecretKey, ristretto::RistrettoSecretKey};
use tari_utilities::ByteArray;
use tari_ootle_transaction::Epoch;
use tari_ootle_wallet_crypto::encryption::{encrypt_with_password, decrypt_with_password};
use serde_json::json;
use ootle_rs::transaction::TransactionSigner;

type Wallet = OotleWallet;
const INDEXER: &str = "https://ootle-indexer-a.tari.com";

fn identity_named(name: &str) -> Result<PrivateKeyProvider> {
    ensure!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'), "invalid identity name");
    fs::create_dir_all(".local")?;
    let pass_path = Path::new(".local/unlock.secret");
    if !pass_path.exists() {
        fs::write(pass_path, RistrettoSecretKey::random(&mut rand::rng()).as_bytes())?;
    }
    let pass = fs::read(pass_path)?;
    let filename = format!(".local/{name}.recovery");
    let path = Path::new(&filename);
    if !path.exists() {
        let secret = OotleSecretKey::random(Network::Esmeralda);
        let data = json!({"version":1,"network":"esmeralda","account":hex::encode(secret.account_secret().as_bytes()),"view":hex::encode(secret.view_only_secret().as_bytes())});
        fs::write(path, encrypt_with_password(&serde_json::to_vec(&data)?, &pass)?)?;
    }
    let data: serde_json::Value = serde_json::from_slice(&decrypt_with_password(&fs::read(path)?, &pass)?)?;
    ensure!(data["network"] == "esmeralda" && data["version"] == 1, "identity context mismatch");
    Ok(PrivateKeyProvider::new(OotleSecretKey::new(Network::Esmeralda,
        RistrettoSecretKey::from_canonical_bytes(&hex::decode(data["account"].as_str().unwrap())?).map_err(|e| anyhow::anyhow!(e.to_string()))?,
        RistrettoSecretKey::from_canonical_bytes(&hex::decode(data["view"].as_str().unwrap())?).map_err(|e| anyhow::anyhow!(e.to_string()))?)))
}

async fn submit(provider: &mut IndexerProvider<Wallet>, unsigned: tari_ootle_transaction::UnsignedTransaction, label: &str) -> Result<()> {
    if Path::new(&format!("evidence/{label}-receipt.json")).exists() {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(format!("evidence/{label}-receipt.json"))?)?;
        ensure!(value["receipt"]["outcome"]=="Commit","{label} saved receipt is not a full main-intent Commit");
        return Ok(());
    }
    let pending_path=format!("evidence/{label}-pending.json");
    if Path::new(&pending_path).exists() {
        let value:serde_json::Value=serde_json::from_slice(&fs::read(&pending_path)?)?;
        let id=tari_ootle_transaction::TransactionId::from_hex(value["transaction_id"].as_str().ok_or_else(||anyhow::anyhow!("bad pending record"))?)?;
        let client=provider.weak_client().upgrade().ok_or_else(||anyhow::anyhow!("indexer connection lost"))?;
        let response=client.get_transaction_receipt(id.into_receipt_address()).await?;
        ensure!(response.receipt.outcome.is_commit(),"{label} main intent did not commit");
        fs::write(format!("evidence/{label}-receipt.json"),serde_json::to_vec_pretty(&json!({"transaction_id":id.to_string(),"receipt":response.receipt}))?)?;
        return Ok(());
    }
    let dry = provider.sign_and_send_dry_run(unsigned.clone()).await?;
    fs::create_dir_all("evidence")?;
    fs::write(format!("evidence/{label}-dry-run.json"), serde_json::to_vec_pretty(&dry)?)?;
    println!("{label} dry-run: {}", dry.finalize.result);
    ensure!(dry.finalize.any_reject().is_none(), "dry-run main intent rejected (see evidence)");
    let tx = provider.wallet().sign_transaction(unsigned).await?;
    let expected_id=tx.calculate_id();
    fs::write(&pending_path,serde_json::to_vec_pretty(&json!({"transaction_id":expected_id.to_string(),"status":"prepared-or-submitted"}))?)?;
    let pending = provider.send_transaction(tx).await?;
    let txid = pending.tx_id().to_string();
    ensure!(pending.tx_id()==expected_id,"submitted transaction ID mismatch");
    let outcome = pending.watch().await?;
    println!("{label}: {txid} {outcome}");
    ensure!(outcome.is_commit(), "main intent did not commit: {outcome}");
    let receipt = pending.get_receipt().await?;
    ensure!(receipt.outcome.is_commit(),"{label} receipt is not a full main-intent Commit");
    fs::write(format!("evidence/{label}-receipt.json"), serde_json::to_vec_pretty(&json!({"transaction_id":txid,"receipt":receipt}))?)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let action = std::env::args().nth(1).unwrap_or_else(|| "help".into());
    if action == "help" { println!("threshold bootstrap | publish | demo | missed | partial | scale8 | scale16 | scale32 | partial-late-check | fee-only-check | direct-bypass-check | inspect PACKAGE.recovery UNLOCK.secret [INDEXER_URL] | recover PACKAGE.recovery UNLOCK.secret [INDEXER_URL] | verify-refund PACKAGE.recovery UNLOCK.secret [INDEXER_URL] | respend-refund PACKAGE.recovery UNLOCK.secret [INDEXER_URL]\nTestnet only. Portable commands read only the supplied package and unlock key."); return Ok(()); }
    if action == "recover" || action == "inspect" { return demo::recover_command().await; }
    if action == "verify-refund" || action == "respend-refund" { return demo::verify_refund_command(action == "respend-refund").await; }
    let signer = identity_named("operator")?;
    let wallet = OotleWallet::from(signer.clone());
    let mut provider = ProviderBuilder::new().wallet(wallet).connect(INDEXER).await?;
    ensure!(provider.get_network().await? == Network::Esmeralda, "testnet only");
    println!("Test account: {}", signer.address().to_account_address());
    let max = Epoch(provider.get_epoch().await?.as_u64()+10);
    match action.as_str() {
        "bootstrap" => {
            let unsigned = IFaucet::new(&provider,max).take_faucet_funds().pay_fee(1_000_000u64).prepare().await?;
            submit(&mut provider, unsigned, "bootstrap").await?;
        },
        "publish" => {
            let code = fs::read("target/wasm32-unknown-unknown/release/threshold_template.wasm")?;
            let unsigned = IAccount::new(&provider,max).pay_fee(100_000_000u64).publish_template(code).prepare().await?;
            submit(&mut provider,unsigned,"publish").await?;
        },
        "demo" => demo::run(&mut provider, &signer).await?,
        "missed" => demo::run_missed(&mut provider, &signer).await?,
        "partial" => demo::run_partial(&mut provider, &signer).await?,
        "scale8" => demo::run_scale(&mut provider, &signer, 8).await?,
        "scale16" => demo::run_scale(&mut provider, &signer, 16).await?,
        "scale32" => demo::run_scale(&mut provider, &signer, 32).await?,
        "partial-late-check" => demo::run_partial_late_check(&mut provider, &signer).await?,
        "fee-only-check" => demo::run_fee_only_check(&mut provider, &signer, false).await?,
        "direct-bypass-check" => demo::run_fee_only_check(&mut provider, &signer, true).await?,
        _ => anyhow::bail!("unknown command"),
    }
    Ok(())
}

#[path = "../network_demo.rs"]
mod demo;
