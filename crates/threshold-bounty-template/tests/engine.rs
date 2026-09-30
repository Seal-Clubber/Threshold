//! Runs the compiled bounty WASM in the Ootle engine. No network submission.
use ootle_byte_type::ToByteType;
use ootle_rs::keys::OotleSecretKey;
use tari_engine_types::{
    substate::SubstateId,
    virtual_substate::{VirtualSubstate, VirtualSubstateId},
};
use tari_ootle_transaction::{Network, args};
use tari_ootle_wallet_crypto::{MaskAndValue, StealthInputWitness, StealthOutputWitness, stealth};
use tari_template_lib::{
    prelude::*,
    types::{UtxoAddress, constants::TARI_TOKEN},
};
use tari_template_test_tooling::TemplateTest;
use threshold_bounty_template::{AwardInput, Status, Tier};
use threshold_client::{check_aggregate, escrow_input, private_output};

fn hash(n: u8) -> Hash32 {
    Hash32::from_array([n; 32])
}

fn make_bounty(
    test: &mut TemplateTest,
    funder: RistrettoPublicKeyBytes,
    issue: u64,
    reviewers: Vec<RistrettoPublicKeyBytes>,
    quorum: u32,
) -> ComponentAddress {
    test.call_function(
        "BountyEscrow",
        "new",
        args![
            String::from("tari-project/tari-ootle"),
            issue,
            hash(3),
            Tier::S,
            funder,
            reviewers,
            quorum,
            Vec::<RistrettoPublicKeyBytes>::new(),
            0u32
        ],
        vec![],
    )
}

fn fund_bounty(
    test: &mut TemplateTest,
    component: ComponentAddress,
    funder_account: ComponentAddress,
    funder_proof: NonFungibleAddress,
    funder_secret: &tari_crypto::ristretto::RistrettoSecretKey,
) -> MaskAndValue {
    let wallet = OotleSecretKey::random(Network::LocalNet);
    let output =
        private_output(&wallet.to_address(), Tier::S.microtari(), Some(component)).unwrap();
    let cell = MaskAndValue {
        mask: output.witness.mask.clone(),
        value: output.witness.amount,
    };
    let mask = check_aggregate(&[cell.clone()], Tier::S.microtari()).unwrap();
    let transfer = stealth::create_transfer_statement(
        Vec::<StealthInputWitness>::new(),
        Amount::from(Tier::S.microtari()),
        [&output],
        Amount::zero(),
    )
    .unwrap();
    let tx = test
        .transaction()
        .with_auto_fill_inputs()
        .call_method(
            funder_account,
            "withdraw",
            args![TARI_TOKEN, Tier::S.microtari()],
        )
        .put_last_instruction_output_on_workspace("funds")
        .call_method(component, "fund", args![Workspace("funds"), transfer, mask])
        .build_and_seal(funder_secret);
    test.execute_expect_success(tx, vec![funder_proof]);
    cell
}

fn escrow_source(cell: &MaskAndValue) -> SubstateId {
    SubstateId::Utxo(UtxoAddress::new(
        TARI_TOKEN,
        cell.to_commitment().to_byte_type().into(),
    ))
}

#[test]
fn no_deadline_full_no_work_refund_and_reviewed_split_claims() {
    let mut test = TemplateTest::new(env!("CARGO_MANIFEST_DIR"), ["."]);
    let (funder_account, funder_proof, funder_secret, funder_public) =
        test.create_funded_account_with_keypair();
    // Each local test account gets 1,000 TARI once; combine fifteen distinct accounts.
    for _ in 1..15 {
        let (donor_account, donor_proof, donor_secret) = test.create_funded_account();
        let top_up = test
            .transaction()
            .with_auto_fill_inputs()
            .call_method(
                donor_account,
                "withdraw",
                args![TARI_TOKEN, 1_000_000_000u64],
            )
            .put_last_instruction_output_on_workspace("top_up")
            .call_method(funder_account, "deposit", args![Workspace("top_up")])
            .build_and_seal(&donor_secret);
        test.execute_expect_success(top_up, vec![donor_proof]);
    }
    let funder = funder_public.to_byte_type();
    let manager = test.public_key().to_byte_type();
    let abandoned = make_bounty(&mut test, funder, 1931, vec![manager], 1);
    let abandoned_cell = fund_bounty(
        &mut test,
        abandoned,
        funder_account,
        funder_proof.clone(),
        &funder_secret,
    );

    let premature = test
        .transaction()
        .call_method(
            abandoned,
            "refund_no_work",
            args![
                stealth::create_transfer_statement(
                    [escrow_input(abandoned_cell.clone(), abandoned).unwrap()],
                    Amount::zero(),
                    std::iter::empty::<&StealthOutputWitness>(),
                    Amount::from(Tier::S.microtari()),
                )
                .unwrap()
            ],
        )
        .build_and_seal(&funder_secret);
    assert!(
        test.execute_expect_failure(premature, vec![])
            .to_string()
            .contains("not closed for no work")
    );

    // No deadline: even a far-future consensus epoch does not take away the reviewer decision.
    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(1_000_000),
    );
    let wrong_reviewer = test
        .transaction()
        .call_method(abandoned, "mark_no_work", args![hash(4)])
        .build_and_seal(&funder_secret);
    assert!(
        test.execute_expect_failure(wrong_reviewer, vec![])
            .to_string()
            .contains("not a reviewer")
    );
    test.call_method::<()>(abandoned, "mark_no_work", args![hash(4)], vec![]);
    let refund_transfer = stealth::create_transfer_statement(
        [escrow_input(abandoned_cell.clone(), abandoned).unwrap()],
        Amount::zero(),
        std::iter::empty::<&StealthOutputWitness>(),
        Amount::from(Tier::S.microtari()),
    )
    .unwrap();
    let refund = test
        .transaction()
        .with_auto_fill_inputs()
        .add_input(escrow_source(&abandoned_cell))
        .call_method(abandoned, "refund_no_work", args![refund_transfer])
        .put_last_instruction_output_on_workspace("refund")
        .call_method(funder_account, "deposit", args![Workspace("refund")])
        .build_and_seal(&funder_secret);
    test.execute_expect_success(refund, vec![funder_proof.clone()]);
    let (status, remaining): (Status, u64) = test.call_method(abandoned, "status", args![], vec![]);
    assert_eq!((status, remaining), (Status::Refunded, 0));

    let (second_reviewer_secret, second_reviewer_public) = test.new_key_pair(42);
    let (_, third_reviewer_public) = test.new_key_pair(43);
    let completed = make_bounty(&mut test, funder, 1949, vec![manager], 1);
    test.call_method::<()>(
        completed,
        "set_reviewers",
        args![
            vec![
                manager,
                second_reviewer_public.to_byte_type(),
                third_reviewer_public.to_byte_type()
            ],
            2u32
        ],
        vec![],
    );
    let first_cell = fund_bounty(
        &mut test,
        completed,
        funder_account,
        funder_proof.clone(),
        &funder_secret,
    );
    let frozen = test
        .transaction()
        .call_method(completed, "revise_scope", args![hash(5)])
        .build_and_seal(test.secret_key());
    assert!(
        test.execute_expect_failure(frozen, vec![])
            .to_string()
            .contains("funded terms are frozen")
    );
    let frozen_reviewers = test
        .transaction()
        .call_method(completed, "set_reviewers", args![vec![manager], 1u32])
        .build_and_seal(test.secret_key());
    assert!(
        test.execute_expect_failure(frozen_reviewers, vec![])
            .to_string()
            .contains("funded terms are frozen")
    );

    let (alice_account, alice_proof, alice_secret, alice_public) =
        test.create_funded_account_with_keypair();
    let (bob_account, bob_proof, bob_secret, bob_public) =
        test.create_funded_account_with_keypair();
    test.call_method::<()>(
        completed,
        "propose_award",
        args![
            vec![
                AwardInput {
                    recipient: alice_public.to_byte_type(),
                    amount: 9_000_000_000,
                    pr_hash: hash(10)
                },
                AwardInput {
                    recipient: bob_public.to_byte_type(),
                    amount: 6_000_000_000,
                    pr_hash: hash(10)
                },
            ],
            hash(11)
        ],
        vec![],
    );
    let (status, remaining): (Status, u64) = test.call_method(completed, "status", args![], vec![]);
    assert_eq!((status, remaining), (Status::Proposed, Tier::S.microtari()));
    let (keys, quorum, approvals, _): (
        Vec<RistrettoPublicKeyBytes>,
        u32,
        Vec<bool>,
        Vec<Option<Hash32>>,
    ) = test.call_method(completed, "reviewers", args![], vec![]);
    assert_eq!(
        (keys.len(), quorum, approvals),
        (3, 2, vec![true, false, false])
    );
    let second_vote = test
        .transaction()
        .call_method(completed, "approve_award", args![])
        .build_and_seal(&second_reviewer_secret);
    test.execute_expect_success(second_vote, vec![]);
    let (status, remaining): (Status, u64) = test.call_method(completed, "status", args![], vec![]);
    assert_eq!((status, remaining), (Status::Awarded, Tier::S.microtari()));
    let too_late = test
        .transaction()
        .call_method(completed, "mark_no_work", args![hash(12)])
        .build_and_seal(test.secret_key());
    assert!(
        test.execute_expect_failure(too_late, vec![])
            .to_string()
            .contains("award already final")
    );

    let wallet = OotleSecretKey::random(Network::LocalNet);
    let remainder_output =
        private_output(&wallet.to_address(), 6_000_000_000, Some(completed)).unwrap();
    let remainder_cell = MaskAndValue {
        mask: remainder_output.witness.mask.clone(),
        value: remainder_output.witness.amount,
    };
    let alice_transfer = stealth::create_transfer_statement(
        [escrow_input(first_cell.clone(), completed).unwrap()],
        Amount::zero(),
        [&remainder_output],
        Amount::from(9_000_000_000u64),
    )
    .unwrap();
    let wrong_claim = test
        .transaction()
        .with_auto_fill_inputs()
        .add_input(escrow_source(&first_cell))
        .call_method(completed, "claim", args![0u32, alice_transfer.clone()])
        .build_and_seal(&funder_secret);
    assert!(
        test.execute_expect_failure(wrong_claim, vec![])
            .to_string()
            .contains("recipient must sign")
    );
    let alice_claim = test
        .transaction()
        .with_auto_fill_inputs()
        .add_input(escrow_source(&first_cell))
        .call_method(completed, "claim", args![0u32, alice_transfer])
        .put_last_instruction_output_on_workspace("payout")
        .call_method(alice_account, "deposit", args![Workspace("payout")])
        .build_and_seal(&alice_secret);
    test.execute_expect_success(alice_claim, vec![alice_proof]);

    let bob_transfer = stealth::create_transfer_statement(
        [escrow_input(remainder_cell.clone(), completed).unwrap()],
        Amount::zero(),
        std::iter::empty::<&StealthOutputWitness>(),
        Amount::from(6_000_000_000u64),
    )
    .unwrap();
    let bob_claim = test
        .transaction()
        .with_auto_fill_inputs()
        .add_input(escrow_source(&remainder_cell))
        .call_method(completed, "claim", args![1u32, bob_transfer])
        .put_last_instruction_output_on_workspace("payout")
        .call_method(bob_account, "deposit", args![Workspace("payout")])
        .build_and_seal(&bob_secret);
    test.execute_expect_success(bob_claim, vec![bob_proof]);
    let (status, remaining): (Status, u64) = test.call_method(completed, "status", args![], vec![]);
    assert_eq!((status, remaining), (Status::Paid, 0));
}
