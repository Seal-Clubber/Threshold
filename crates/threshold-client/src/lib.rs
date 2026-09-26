//! Reusable real Ootle cryptography, independent of the coordinator.
use anyhow::{Result, ensure};
use ootle_rs::{Address, Network, keys::OotleSecretKey};
use ootle_byte_type::{ToByteType, ConvertFromByteType};
use tari_utilities::ByteArray;
use tari_crypto::{keys::{PublicKey, SecretKey}, ristretto::{RistrettoSecretKey, RistrettoPublicKey}};
use tari_ootle_wallet_crypto::{MaskAndValue, OutputWitness, StealthOutputWitness, StealthInputWitness, encrypted_data, kdfs, stealth};
use tari_template_lib::prelude::*;
use tari_template_lib::types::stealth::{SpendAuthorization, StealthUnspentOutput};
use threshold_template::{escrow_leaf, escrow_root};

pub fn private_output(address: &Address, value: u64, lock: Option<ComponentAddress>) -> Result<StealthOutputWitness> {
    ensure!(value > 0, "zero output");
    let nonce = RistrettoSecretKey::random(&mut ::rand::rng());
    let mask = RistrettoSecretKey::random(&mut ::rand::rng());
    let view = RistrettoPublicKey::convert_from_byte_type(address.view_only_key()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let owner = RistrettoPublicKey::convert_from_byte_type(address.account_public_key()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let encryption = kdfs::encrypted_data_dh_kdf_aead(&nonce, &view);
    let auth = match lock {
        Some(component) => SpendAuthorization::Script(escrow_root(component)),
        None => SpendAuthorization::Key(kdfs::owner_stealth_dh_stealth_address(address.network(), &owner, &nonce).to_byte_type()),
    };
    Ok(StealthOutputWitness {
        witness: OutputWitness {
            amount: value, mask: mask.clone(), minimum_value_promise: if lock.is_some() { 1 } else { 0 },
            sender_public_nonce: RistrettoPublicKey::from_secret_key(&nonce),
            encrypted_data: encrypted_data::encrypt_data(value, &mask, &encryption, None)?,
            resource_view_key: None,
        },
        auth,
        tag: kdfs::utxo_tag_stealth_dh(address.network(), &view, &nonce, &tari_template_lib::types::constants::TARI_TOKEN),
    })
}

pub fn decrypt_output(secret: &OotleSecretKey, output: &StealthUnspentOutput) -> Result<MaskAndValue> {
    let nonce = RistrettoPublicKey::convert_from_byte_type(&output.output.sender_public_nonce).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let encryption = kdfs::encrypted_data_dh_kdf_aead(secret.view_only_secret(), &nonce);
    Ok(encrypted_data::unblind_output(&output.output.commitment, &output.output.encrypted_data, &encryption, false)?.mask_and_value)
}

pub fn escrow_input(cell: MaskAndValue, component: ComponentAddress) -> Result<StealthInputWitness> {
    let leaf = escrow_leaf(component);
    let (witness, root) = stealth::script_path_witness(&[leaf.clone()], &leaf)?;
    Ok(StealthInputWitness::with_script_path(cell, witness, root))
}

pub fn private_transfer(inputs: Vec<StealthInputWitness>, outputs: &[StealthOutputWitness]) -> Result<StealthTransferStatement> {
    Ok(stealth::create_transfer_statement(inputs, Amount::zero(), outputs.iter(), Amount::zero())?)
}

pub fn check_aggregate(cells: &[MaskAndValue], expected: u64) -> Result<Scalar32Bytes> {
    ensure!(!cells.is_empty(), "empty role");
    let sum = cells.iter().try_fold(0u64, |a, c| a.checked_add(c.value)).ok_or_else(|| anyhow::anyhow!("amount overflow"))?;
    ensure!(sum == expected, "exact role budget not met");
    Ok(Scalar32Bytes::from_bytes(cells.iter().fold(RistrettoSecretKey::default(), |a, c| a + &c.mask).as_bytes())?)
}

/// Default allocation only; actual per-pledge refund rights are its chosen cells.
pub fn apportion(value: u64, weights: &[u64]) -> Result<Vec<u64>> {
    let total = weights.iter().try_fold(0u64, |a, w| a.checked_add(*w)).ok_or_else(|| anyhow::anyhow!("weight overflow"))?;
    ensure!(total > 0 && !weights.is_empty() && weights.iter().all(|w| *w > 0), "positive weights required");
    let mut parts = weights.iter().map(|w| (u128::from(value)*u128::from(*w)/u128::from(total)) as u64).collect::<Vec<_>>();
    let remainder = value - parts.iter().sum::<u64>();
    // Largest fractional remainder, stable stage index tie-break.
    let mut order = (0..weights.len()).collect::<Vec<_>>();
    order.sort_by_key(|i| (std::cmp::Reverse(u128::from(value)*u128::from(weights[*i])%u128::from(total)), *i));
    for i in order.into_iter().take(remainder as usize) { parts[i] += 1; }
    Ok(parts)
}

pub fn demo_secret() -> OotleSecretKey { OotleSecretKey::random(Network::Esmeralda) }

