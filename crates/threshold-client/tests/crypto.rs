//! Real upstream proof verification; these tests do NOT execute a ledger or WASM.
use threshold_client::*;
use threshold_template::*;
use tari_template_lib::prelude::*;
use tari_template_lib::types::ObjectKey;
use tari_crypto::{commitment::HomomorphicCommitmentFactory, ristretto::{RistrettoPublicKey, RistrettoSecretKey}, keys::PublicKey};
use tari_ootle_wallet_crypto::{MaskAndValue, StealthInputWitness, stealth, encryption::*};
use tari_engine_types::{crypto::get_commitment_factory, stealth::{condition_leaf_hash, validate_transfer}};
use tari_template_lib::types::stealth::SpendAuthorization;
use ootle_byte_type::ToByteType;
use tari_utilities::ByteArray;

fn component() -> ComponentAddress { ComponentAddress::new(ObjectKey::from_array([7;32])) }

#[test]
fn script_root_matches_pinned_engine() {
    assert_eq!(escrow_root(component()), condition_leaf_hash(&escrow_leaf(component())));
}

#[test]
fn fixed_value_generator_matches_engine_factory() {
    assert_eq!(get_commitment_factory().commit_value(&RistrettoSecretKey::default(), 1).to_byte_type().into_array(), VALUE_GENERATOR);
}

#[test]
fn exact_aggregate_opening_rejects_sponsor_substitution_and_shortfall() {
    let me = demo_secret();
    let outputs = [private_output(&me.to_address(), 1500, Some(component())).unwrap(), private_output(&me.to_address(), 4500, Some(component())).unwrap()];
    let cells = outputs.iter().map(|o| MaskAndValue { mask: o.witness.mask.clone(), value: o.witness.amount }).collect::<Vec<_>>();
    let mask = check_aggregate(&cells, 6000).unwrap();
    assert!(check_aggregate(&cells, 10000).is_err());
    assert!(check_aggregate(&cells, 6001).is_err());
    let expected = get_commitment_factory().commit_value(&RistrettoSecretKey::from_canonical_bytes(mask.as_bytes()).unwrap(), 6000);
    let actual = cells.iter().fold(RistrettoPublicKey::default(), |a,c| a + c.to_commitment().as_public_key());
    assert_eq!(&actual, expected.as_public_key());
}

#[test]
fn native_range_and_balance_proofs_and_recipient_decrypt_respend() {
    let alice = demo_secret(); let bob = demo_secret();
    let output = private_output(&bob.to_address(), 6_000_000_000, None).unwrap();
    let mint = stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(), Amount::from(6_000_000_000u64), [&output], Amount::zero()).unwrap();
    validate_transfer(&mint, None).unwrap();
    let received = &mint.stealth_outputs()[0];
    assert!(decrypt_output(&alice, received).is_err());
    let clear = decrypt_output(&bob, received).unwrap();
    assert_eq!(clear.value, 6_000_000_000);
    let nonce = RistrettoPublicKey::from_canonical_bytes(received.output.sender_public_nonce.as_bytes()).unwrap();
    let spend_secret = tari_ootle_wallet_crypto::kdfs::owner_stealth_dh_secret(bob.network(), bob.account_secret(), &nonce);
    assert_eq!(received.auth, SpendAuthorization::Key(RistrettoPublicKey::from_secret_key(&spend_secret).to_byte_type()));
    let next = private_output(&alice.to_address(), clear.value, None).unwrap();
    let transfer = private_transfer(vec![StealthInputWitness::new(clear)], &[next]).unwrap();
    validate_transfer(&transfer, None).unwrap();
    assert_eq!(decrypt_output(&alice, &transfer.stealth_outputs()[0]).unwrap().value, 6_000_000_000);
    let mut inflation = transfer.clone(); inflation.outputs_statement.revealed_output_amount = Amount::from(1u64);
    assert!(validate_transfer(&inflation, None).is_err());
}

#[test]
fn payout_validation_rejects_extra_outputs_public_flow_missing_and_repeated_inputs() {
    let me = demo_secret();
    let locked = private_output(&me.to_address(), 700, Some(component())).unwrap();
    let cell = MaskAndValue { mask: locked.witness.mask, value: 700 };
    let expected = vec![cell.to_commitment().to_byte_type()];
    let input = escrow_input(cell, component()).unwrap();
    let output = private_output(&me.to_address(), 700, None).unwrap();
    let good = private_transfer(vec![input], &[output]).unwrap();
    validate_spend(&expected, &good);
    validate_transfer(&good, None).unwrap();
    for variant in 0..6 {
        let mut bad = good.clone();
        match variant {
            0 => bad.outputs_statement.outputs.push(bad.outputs_statement.outputs[0].clone()),
            1 => bad.outputs_statement.revealed_output_amount = Amount::from(1u64),
            2 => bad.inputs_statement.inputs.clear(),
            3 => bad.inputs_statement.inputs.push(bad.inputs_statement.inputs[0].clone()),
            4 => bad.inputs_statement.revealed_amount = Amount::from(1u64),
            _ => bad.outputs_statement.outputs[0].auth = SpendAuthorization::KeyAndScript { spend_key: *me.to_address().account_public_key(), condition_root: escrow_root(component()) },
        }
        assert!(std::panic::catch_unwind(|| validate_spend(&expected, &bad)).is_err(), "variant {variant}");
    }
}

#[test]
fn v1_payout_shape_does_not_prove_recipient_decryption() {
    let recipient=demo_secret();
    let alternate=demo_secret();
    let locked=private_output(&recipient.to_address(),700,Some(component())).unwrap();
    let cell=MaskAndValue{mask:locked.witness.mask,value:700};
    let expected=vec![cell.to_commitment().to_byte_type()];
    let redirected=private_transfer(vec![escrow_input(cell,component()).unwrap()],
        &[private_output(&alternate.to_address(),700,None).unwrap()]).unwrap();
    validate_spend(&expected,&redirected);
    validate_transfer(&redirected,None).unwrap();
    assert!(decrypt_output(&recipient,&redirected.stealth_outputs()[0]).is_err());
    assert_eq!(decrypt_output(&alternate,&redirected.stealth_outputs()[0]).unwrap().value,700);
}

#[test]
fn portable_encryption_rejects_wrong_password_and_tampering() {
    let payload = b"test-only recovery material";
    let encrypted = encrypt_with_password(payload, b"long-test-only-password").unwrap();
    assert_eq!(&*decrypt_with_password(&encrypted, b"long-test-only-password").unwrap(), payload);
    assert!(decrypt_with_password(&encrypted, b"wrong-password").is_err());
    let mut tampered = encrypted; tampered[5] ^= 1;
    assert!(decrypt_with_password(&tampered, b"long-test-only-password").is_err());
}

#[test]
fn integer_apportionment_conserves_dust_and_large_values() {
    for value in [1,2,3,7,999,6_000_000_000,u64::MAX] {
        let p = apportion(value, &[2,4,4]).unwrap();
        assert_eq!(p.iter().map(|v| *v as u128).sum::<u128>(), value as u128);
    }
    assert_eq!(apportion(7, &[2,4,4]).unwrap(), vec![1,3,3]);
    assert!(apportion(10, &[0,0]).is_err());
}

#[test]
fn terms_reject_zero_quorum_duplicate_reviewers_and_deadline_overlap() {
    let a = demo_secret().to_address(); let b = demo_secret().to_address();
    let stage = Stage { leg: 0, community: 6, sponsor: 4, recipient: *a.account_public_key(), reviewers: vec![*b.account_public_key()], quorum: 1, deadline: 12, upfront: false };
    let good = Terms { version:1, nonce:Hash32::from_array([1;32]), resource: tari_template_lib::types::constants::TARI_TOKEN, funding_deadline:10, sponsor_key:*a.account_public_key(), stages:vec![stage] };
    validate_terms(&good, 9);
    assert!(std::panic::catch_unwind(|| validate_terms(&good,10)).is_err());
    for variant in 0..3 {
        let mut bad=good.clone();
        match variant { 0 => bad.stages[0].quorum=0, 1 => bad.stages[0].reviewers.push(*b.account_public_key()), _ => bad.stages[0].deadline=10 }
        assert!(std::panic::catch_unwind(|| validate_terms(&bad,9)).is_err());
    }
}
