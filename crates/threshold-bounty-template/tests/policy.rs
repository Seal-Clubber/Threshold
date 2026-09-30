use tari_template_lib::prelude::{Hash32, RistrettoPublicKeyBytes};
use threshold_bounty_template::{
    AwardInput, Tier, validate_awards, validate_issue, validate_reviewers,
};

fn key(n: u8) -> RistrettoPublicKeyBytes {
    RistrettoPublicKeyBytes::from([n; 32])
}
fn hash(n: u8) -> Hash32 {
    Hash32::from_array([n; 32])
}

#[test]
fn fixed_tiers_and_split_total_are_enforced() {
    assert_eq!(
        [Tier::S, Tier::M, Tier::L, Tier::XL].map(Tier::tari),
        [15_000, 60_000, 150_000, 450_000]
    );
    let valid = vec![
        AwardInput {
            recipient: key(1),
            amount: 9_000_000_000,
            pr_hash: hash(11),
        },
        AwardInput {
            recipient: key(2),
            amount: 6_000_000_000,
            pr_hash: hash(11),
        },
    ];
    validate_awards(Tier::S, &valid, hash(12));
    let mut invalid = valid.clone();
    invalid[1].amount -= 1;
    assert!(std::panic::catch_unwind(|| validate_awards(Tier::S, &invalid, hash(12))).is_err());
    invalid = valid;
    invalid[1].recipient = key(1);
    assert!(std::panic::catch_unwind(|| validate_awards(Tier::S, &invalid, hash(12))).is_err());
}

#[test]
fn issue_identity_requires_public_tari_repo_and_scope_digest() {
    validate_issue("tari-project/tari-ootle", 1931, hash(1));
    assert!(
        std::panic::catch_unwind(|| validate_issue("other-project/tari-ootle", 1931, hash(1)))
            .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| validate_issue("tari-project/tari-ootle", 0, hash(1))).is_err()
    );
    assert!(
        std::panic::catch_unwind(|| validate_issue("tari-project/tari-ootle", 1931, hash(0)))
            .is_err()
    );
}

#[test]
fn council_review_quorum_supports_one_of_one_one_of_three_and_two_of_three() {
    let manager = key(1);
    validate_reviewers(manager, &[manager], 1);
    validate_reviewers(manager, &[manager, key(2), key(3)], 1);
    validate_reviewers(manager, &[manager, key(2), key(3)], 2);
    assert!(
        std::panic::catch_unwind(|| validate_reviewers(manager, &[manager, key(2), key(3)], 4))
            .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| validate_reviewers(manager, &[manager, manager], 1)).is_err()
    );
    assert!(std::panic::catch_unwind(|| validate_reviewers(manager, &[key(2)], 1)).is_err());
}
