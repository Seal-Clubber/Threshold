//! Lightweight term/serialization checks only. These do not run an Ootle engine or transaction.
use tari_template_lib::{prelude::*, types::constants::TARI_TOKEN};
use threshold_poc_core::{CampaignProfile::*, *};

// Synthetic byte fixtures for pure validation; never signing identities or network wallets.
fn key(value: u8) -> RistrettoPublicKeyBytes {
    RistrettoPublicKeyBytes::from([value; 32])
}
fn config(profile: CampaignProfile) -> ExampleConfig {
    let count = match profile {
        Infrastructure => 4,
        Community => 2,
        _ => 3,
    };
    let mut recipients = vec![key(1); count];
    if profile == Audit {
        recipients[count - 1] = key(2);
    }
    ExampleConfig {
        nonce: Hash32::from_array([7; 32]),
        resource: TARI_TOKEN,
        funding_deadline: 20,
        stage_spacing: 10,
        sponsor_key: if profile == Community {
            None
        } else {
            Some(key(3))
        },
        recipients,
        reviewers: if profile == Grant {
            vec![key(4), key(5), key(6), key(7), key(8)]
        } else {
            vec![key(4), key(5), key(6)]
        },
    }
}

#[test]
fn examples_have_expected_budgets_and_round_trip() {
    for (profile, community, sponsor, stages) in [
        (Exchange, 600, 400, 3),
        (Audit, 350, 150, 3),
        (Infrastructure, 180, 60, 4),
        (Grant, 100, 100, 3),
        (Community, 100, 0, 2),
    ] {
        let terms = example_terms(profile, config(profile));
        validate_terms(&terms, 10);
        assert_eq!(terms.stages.len(), stages);
        assert_eq!(
            terms.stages.iter().map(|s| s.community).sum::<u64>(),
            community * 1_000_000
        );
        assert_eq!(
            terms.stages.iter().map(|s| s.sponsor).sum::<u64>(),
            sponsor * 1_000_000
        );
        assert!(terms.stages[0].upfront && terms.stages[0].reviewers.is_empty());
        assert!(terms.stages[1..].iter().all(|s| !s.upfront && s.quorum > 0));
        let bytes = minicbor::to_vec(&terms).unwrap();
        let decoded: Terms = minicbor::decode(&bytes).unwrap();
        assert_eq!(decoded.profile, profile);
        assert_eq!(decoded.mode, terms.mode);
        assert_eq!(decoded.sponsor_key, terms.sponsor_key);
        assert_eq!(minicbor::to_vec(&decoded).unwrap(), bytes);
    }
}

#[test]
fn funding_modes_reject_ambiguous_terms() {
    let terms = example_terms(Community, config(Community));
    assert_eq!(funding_roles(terms.mode), 1);
    let mut invalid = terms.clone();
    invalid.sponsor_key = Some(key(3));
    assert!(std::panic::catch_unwind(|| validate_terms(&invalid, 10)).is_err());
    invalid = terms.clone();
    invalid.stages[0].sponsor = 1;
    assert!(std::panic::catch_unwind(|| validate_terms(&invalid, 10)).is_err());
    let mut matched = example_terms(Exchange, config(Exchange));
    assert_eq!(funding_roles(matched.mode), 2);
    matched.stages[0].sponsor = 0;
    assert!(std::panic::catch_unwind(|| validate_terms(&matched, 10)).is_err());
    assert!(std::panic::catch_unwind(|| validate_profile(&terms, Exchange)).is_err());
}

#[test]
fn community_mode_rejects_sponsor_pledges_and_matched_mode_binds_the_key() {
    let community = example_terms(Community, config(Community));
    validate_pledge_role(&community, &key(9), false);
    assert!(std::panic::catch_unwind(|| validate_pledge_role(&community, &key(9), true)).is_err());
    let matched = example_terms(Exchange, config(Exchange));
    validate_pledge_role(&matched, &key(3), true);
    validate_pledge_role(&matched, &key(9), false);
    assert!(std::panic::catch_unwind(|| validate_pledge_role(&matched, &key(3), false)).is_err());
    assert!(std::panic::catch_unwind(|| validate_pledge_role(&matched, &key(9), true)).is_err());
}

#[test]
fn audit_requires_separate_recipient_and_review_rules_still_apply() {
    let mut audit = config(Audit);
    audit.recipients.fill(key(1));
    assert!(std::panic::catch_unwind(|| example_terms(Audit, audit)).is_err());
    let mut terms = example_terms(Grant, config(Grant));
    terms.stages[1].quorum = 6;
    assert!(std::panic::catch_unwind(|| validate_terms(&terms, 10)).is_err());
    let terms = example_terms(Exchange, config(Exchange));
    assert!(std::panic::catch_unwind(|| validate_terms(&terms, 20)).is_err());
}
