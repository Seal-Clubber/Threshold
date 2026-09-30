//! Experimental Tari-style bounty escrow on Ootle's native stealth TARI resource.
//! GitHub review, merged PRs and contributor identity require human verification.
use blake2::{Blake2b, Digest, digest::consts::U32};
use minicbor::{CborLen, Decode, Encode};
use std::collections::BTreeSet;
use tari_template_lib::{
    prelude::*,
    types::{
        constants::{TARI, TARI_TOKEN},
        stealth::{SpendAuthorization, SpendCondition},
    },
};

pub const VERSION: u32 = 2;
pub const MAX_AWARDEES: usize = 8;
pub const MAX_REVIEWERS: usize = 5;
const VALUE_GENERATOR: [u8; 32] = [
    206, 56, 152, 65, 192, 200, 105, 138, 185, 91, 112, 36, 42, 238, 166, 72, 64, 177, 234, 197,
    246, 68, 183, 208, 8, 172, 5, 135, 207, 71, 29, 112,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, CborLen)]
#[cbor(index_only)]
pub enum Tier {
    #[n(0)]
    S,
    #[n(1)]
    M,
    #[n(2)]
    L,
    #[n(3)]
    XL,
}

impl Tier {
    pub const fn tari(self) -> u64 {
        match self {
            Self::S => 15_000,
            Self::M => 60_000,
            Self::L => 150_000,
            Self::XL => 450_000,
        }
    }

    pub const fn microtari(self) -> u64 {
        self.tari() * TARI
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, CborLen, serde::Serialize, serde::Deserialize,
)]
#[cbor(index_only)]
pub enum Status {
    #[n(0)]
    Open,
    #[n(1)]
    Proposed,
    #[n(2)]
    Awarded,
    #[n(3)]
    NoWork,
    #[n(4)]
    Refunded,
    #[n(5)]
    Paid,
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct Award {
    #[n(0)]
    pub recipient: RistrettoPublicKeyBytes,
    #[n(1)]
    pub amount: u64,
    /// Digest of a public PR/merge reference, supplied by the reviewer.
    #[n(2)]
    pub pr_hash: Hash32,
    #[n(3)]
    pub claimed: bool,
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct AwardInput {
    #[n(0)]
    pub recipient: RistrettoPublicKeyBytes,
    #[n(1)]
    pub amount: u64,
    #[n(2)]
    pub pr_hash: Hash32,
}

pub fn validate_issue(repo: &str, issue: u64, scope_hash: Hash32) {
    assert!(issue > 0, "issue number required");
    assert!(
        repo.starts_with("tari-project/") && repo.len() <= 100,
        "Tari repository required"
    );
    let name = &repo["tari-project/".len()..];
    assert!(
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "invalid repository"
    );
    assert_ne!(
        scope_hash,
        Hash32::from_array([0; 32]),
        "scope digest required"
    );
}

pub fn validate_awards(tier: Tier, awards: &[AwardInput], decision_hash: Hash32) {
    assert_ne!(
        decision_hash,
        Hash32::from_array([0; 32]),
        "decision digest required"
    );
    assert!(
        !awards.is_empty() && awards.len() <= MAX_AWARDEES,
        "award count"
    );
    let mut sum = 0u64;
    let mut recipients = BTreeSet::new();
    for award in awards {
        assert!(award.amount > 0, "positive award required");
        assert_ne!(
            award.pr_hash,
            Hash32::from_array([0; 32]),
            "PR digest required"
        );
        assert!(recipients.insert(award.recipient), "duplicate recipient");
        sum = sum
            .checked_add(award.amount)
            .expect("award amount overflow");
    }
    assert_eq!(sum, tier.microtari(), "split must equal fixed tier price");
}

pub fn validate_reviewers(
    manager: RistrettoPublicKeyBytes,
    reviewers: &[RistrettoPublicKeyBytes],
    quorum: u32,
) {
    assert!(
        !reviewers.is_empty() && reviewers.len() <= MAX_REVIEWERS,
        "reviewer count"
    );
    assert!(reviewers.contains(&manager), "manager must be a reviewer");
    assert!(
        reviewers.iter().collect::<BTreeSet<_>>().len() == reviewers.len(),
        "duplicate reviewer"
    );
    assert!(
        quorum > 0 && quorum as usize <= reviewers.len(),
        "review quorum"
    );
}

pub fn escrow_root(component: ComponentAddress) -> Hash32 {
    let domain = b"com.tari.ootle.engine.v0.ConditionLeaf";
    let leaf = SpendCondition::access_rule(rule!(component(component)));
    let mut digest = Blake2b::<U32>::new();
    digest.update((domain.len() as u64).to_le_bytes());
    digest.update(domain);
    digest.update(0u32.to_le_bytes());
    digest.update(borsh::to_vec(&leaf).expect("condition encoding"));
    Hash32::from_array(digest.finalize().into())
}

fn require_exact_cell(transfer: &StealthTransferStatement, cell: PedersenCommitmentBytes) {
    assert!(
        transfer.revealed_input_amount().is_zero(),
        "public input forbidden"
    );
    assert_eq!(
        transfer.stealth_inputs().len(),
        1,
        "one escrow input required"
    );
    assert_eq!(
        transfer.stealth_inputs()[0].commitment,
        cell,
        "wrong escrow input"
    );
}

#[template]
mod bounty_escrow {
    use super::*;

    pub struct BountyEscrow {
        version: u32,
        repo: String,
        issue: u64,
        scope_hash: Hash32,
        tier: Tier,
        manager: RistrettoPublicKeyBytes,
        reviewers: Vec<RistrettoPublicKeyBytes>,
        review_quorum: u32,
        award_approvals: Vec<bool>,
        no_work_votes: Vec<Option<Hash32>>,
        funder: RistrettoPublicKeyBytes,
        stewards: Vec<RistrettoPublicKeyBytes>,
        steward_quorum: u32,
        approvals: Vec<bool>,
        status: Status,
        cell: Option<PedersenCommitmentBytes>,
        remaining: u64,
        decision_hash: Option<Hash32>,
        awards: Vec<Award>,
    }

    impl BountyEscrow {
        /// Reviewer creates an issue request. The tier and roles are frozen at creation.
        /// The GitHub issue body may be revised until the funding transaction commits.
        pub fn new(
            repo: String,
            issue: u64,
            scope_hash: Hash32,
            tier: Tier,
            funder: RistrettoPublicKeyBytes,
            reviewers: Vec<RistrettoPublicKeyBytes>,
            review_quorum: u32,
            stewards: Vec<RistrettoPublicKeyBytes>,
            steward_quorum: u32,
        ) -> Component<Self> {
            validate_issue(&repo, issue, scope_hash);
            let manager = CallerContext::transaction_signer_public_key();
            validate_reviewers(manager, &reviewers, review_quorum);
            let distinct = stewards.iter().collect::<BTreeSet<_>>();
            assert_eq!(distinct.len(), stewards.len(), "duplicate steward");
            assert!(
                reviewers.iter().all(|key| !stewards.contains(key)),
                "reviewer cannot also be a steward"
            );
            if tier == Tier::XL {
                assert!(
                    stewards.len() >= 2 && stewards.len() <= 7,
                    "XL needs a steward body"
                );
                assert!(
                    steward_quorum >= 2 && steward_quorum as usize <= stewards.len(),
                    "XL steward quorum"
                );
            } else {
                assert!(
                    stewards.is_empty() && steward_quorum == 0,
                    "stewards only for XL"
                );
            }
            Component::new(Self {
                version: VERSION,
                repo,
                issue,
                scope_hash,
                tier,
                manager,
                funder,
                award_approvals: vec![false; reviewers.len()],
                no_work_votes: vec![None; reviewers.len()],
                reviewers,
                review_quorum,
                approvals: vec![false; stewards.len()],
                stewards,
                steward_quorum,
                status: Status::Open,
                cell: None,
                remaining: 0,
                decision_hash: None,
                awards: vec![],
            })
            .with_owner_rule(OwnerRule::None)
            .with_access_rules(AccessRules::allow_all())
            .create()
        }

        pub fn details(
            &self,
        ) -> (
            u32,
            String,
            u64,
            Hash32,
            Tier,
            RistrettoPublicKeyBytes,
            RistrettoPublicKeyBytes,
            Status,
            u64,
        ) {
            (
                self.version,
                self.repo.clone(),
                self.issue,
                self.scope_hash,
                self.tier,
                self.manager,
                self.funder,
                self.status,
                self.remaining,
            )
        }

        pub fn awards(&self) -> Vec<Award> {
            self.awards.clone()
        }
        pub fn status(&self) -> (Status, u64) {
            (self.status, self.remaining)
        }
        pub fn reviewers(
            &self,
        ) -> (
            Vec<RistrettoPublicKeyBytes>,
            u32,
            Vec<bool>,
            Vec<Option<Hash32>>,
        ) {
            (
                self.reviewers.clone(),
                self.review_quorum,
                self.award_approvals.clone(),
                self.no_work_votes.clone(),
            )
        }
        pub fn stewards(&self) -> (Vec<RistrettoPublicKeyBytes>, u32, Vec<bool>) {
            (
                self.stewards.clone(),
                self.steward_quorum,
                self.approvals.clone(),
            )
        }
        pub fn lock_root(&self) -> Hash32 {
            escrow_root(CallerContext::current_component_address())
        }

        pub fn revise_scope(&mut self, scope_hash: Hash32) {
            self.require_manager();
            assert_eq!(self.status, Status::Open, "request closed");
            assert!(self.cell.is_none(), "funded terms are frozen");
            assert_ne!(
                scope_hash,
                Hash32::from_array([0; 32]),
                "scope digest required"
            );
            self.scope_hash = scope_hash;
        }

        /// The council manager may adjust review keys and quorum until funding freezes terms.
        pub fn set_reviewers(&mut self, reviewers: Vec<RistrettoPublicKeyBytes>, quorum: u32) {
            self.require_manager();
            assert_eq!(self.status, Status::Open, "request closed");
            assert!(self.cell.is_none(), "funded terms are frozen");
            validate_reviewers(self.manager, &reviewers, quorum);
            assert!(
                reviewers.iter().all(|key| !self.stewards.contains(key)),
                "reviewer cannot also be a steward"
            );
            self.award_approvals = vec![false; reviewers.len()];
            self.no_work_votes = vec![None; reviewers.len()];
            self.reviewers = reviewers;
            self.review_quorum = quorum;
        }

        /// Funder converts an exact public bucket into one component-locked stealth output.
        pub fn fund(
            &mut self,
            funds: Bucket,
            transfer: StealthTransferStatement,
            mask: Scalar32Bytes,
        ) {
            assert_eq!(
                CallerContext::transaction_signer_public_key(),
                self.funder,
                "only funder"
            );
            assert_eq!(self.status, Status::Open, "request closed");
            assert!(self.cell.is_none(), "already funded");
            assert_eq!(funds.resource_address(), TARI_TOKEN, "native TARI required");
            assert_eq!(
                funds.amount(),
                Amount::from(self.tier.microtari()),
                "exact tier funding required"
            );
            assert!(
                transfer.stealth_inputs().is_empty(),
                "private inputs forbidden in this funding path"
            );
            assert_eq!(
                transfer.revealed_input_amount(),
                Amount::from(self.tier.microtari()),
                "wrong revealed input"
            );
            assert!(
                transfer.revealed_output_amount().is_zero(),
                "funding cannot reveal output"
            );
            assert_eq!(
                transfer.stealth_outputs().len(),
                1,
                "one escrow output required"
            );
            let output = &transfer.stealth_outputs()[0];
            assert_eq!(
                output.auth,
                SpendAuthorization::Script(self.lock_root()),
                "pure component lock required"
            );
            assert_eq!(
                output.output.minimum_value_promise, 1,
                "constant minimum promise required"
            );
            let mut amount_scalar = [0u8; 32];
            amount_scalar[..8].copy_from_slice(&self.tier.microtari().to_le_bytes());
            let expected = intrinsics::ristretto_add(
                &intrinsics::ristretto_mul_base(&mask),
                &intrinsics::ristretto_mul(
                    &RistrettoPublicKeyBytes::from(VALUE_GENERATOR),
                    &Scalar32Bytes::from(amount_scalar),
                ),
            );
            assert_eq!(
                RistrettoPublicKeyBytes::from(output.output.commitment.into_array()),
                expected,
                "wrong tier amount"
            );
            let commitment = output.output.commitment;
            assert!(
                ResourceManager::get(TARI_TOKEN)
                    .stealth_transfer_with_opt_input_bucket(transfer, Some(funds))
                    .is_none(),
                "unexpected public funding"
            );
            self.cell = Some(commitment);
            self.remaining = self.tier.microtari();
        }

        /// Reviewer records the accepted merged PR(s) and the discretionary split.
        /// The hashes are attestations, not GitHub verification by the contract.
        pub fn propose_award(&mut self, awards: Vec<AwardInput>, decision_hash: Hash32) {
            self.require_manager();
            assert_eq!(self.status, Status::Open, "already decided");
            assert!(self.cell.is_some(), "not funded");
            validate_awards(self.tier, &awards, decision_hash);
            self.awards = awards
                .into_iter()
                .map(|a| Award {
                    recipient: a.recipient,
                    amount: a.amount,
                    pr_hash: a.pr_hash,
                    claimed: false,
                })
                .collect();
            self.decision_hash = Some(decision_hash);
            let manager_index = self.reviewer_index();
            self.award_approvals[manager_index] = true;
            self.status = Status::Proposed;
            self.finalize_award_if_ready();
        }

        /// A reviewer approves the manager's frozen award proposal.
        pub fn approve_award(&mut self) {
            assert_eq!(self.status, Status::Proposed, "no award proposal");
            let index = self.reviewer_index();
            assert!(!self.award_approvals[index], "already approved");
            self.award_approvals[index] = true;
            self.finalize_award_if_ready();
        }

        /// XL requires the steward body's threshold approval of the frozen proposal.
        pub fn approve_xl(&mut self) {
            assert_eq!(self.status, Status::Proposed, "no XL proposal");
            assert_eq!(self.tier, Tier::XL, "not XL");
            assert!(self.award_quorum_met(), "review quorum not met");
            let caller = CallerContext::transaction_signer_public_key();
            let index = self
                .stewards
                .iter()
                .position(|key| *key == caller)
                .expect("not a steward");
            assert!(!self.approvals[index], "already approved");
            self.approvals[index] = true;
            if self.approvals.iter().filter(|approved| **approved).count()
                >= self.steward_quorum as usize
            {
                self.status = Status::Awarded;
            }
        }

        /// Before a final award, reviewer may close a no-work bounty at any epoch.
        /// The original funder then claims the complete untouched escrow.
        pub fn mark_no_work(&mut self, reason_hash: Hash32) {
            let index = self.reviewer_index();
            assert!(
                matches!(self.status, Status::Open | Status::Proposed),
                "award already final"
            );
            assert!(self.cell.is_some(), "not funded");
            assert_ne!(
                reason_hash,
                Hash32::from_array([0; 32]),
                "reason digest required"
            );
            assert_ne!(
                self.no_work_votes[index],
                Some(reason_hash),
                "already voted no work"
            );
            self.no_work_votes[index] = Some(reason_hash);
            if self
                .no_work_votes
                .iter()
                .filter(|vote| **vote == Some(reason_hash))
                .count()
                >= self.review_quorum as usize
            {
                self.awards.clear();
                self.decision_hash = Some(reason_hash);
                self.status = Status::NoWork;
            }
        }

        pub fn refund_no_work(&mut self, transfer: StealthTransferStatement) -> Bucket {
            assert_eq!(
                CallerContext::transaction_signer_public_key(),
                self.funder,
                "only original funder"
            );
            assert_eq!(self.status, Status::NoWork, "not closed for no work");
            let cell = self.cell.expect("not funded");
            assert_eq!(
                self.remaining,
                self.tier.microtari(),
                "full amount no longer available"
            );
            require_exact_cell(&transfer, cell);
            assert!(
                transfer.stealth_outputs().is_empty(),
                "refund cannot redirect a private output"
            );
            assert_eq!(
                transfer.revealed_output_amount(),
                Amount::from(self.remaining),
                "full refund required"
            );
            let bucket = ResourceManager::get(TARI_TOKEN)
                .stealth_transfer(transfer)
                .expect("refund bucket required");
            self.cell = None;
            self.remaining = 0;
            self.status = Status::Refunded;
            bucket
        }

        /// An awardee signs their own claim and receives a public bucket. A partial
        /// claim leaves one component-locked private output for the other awardees.
        pub fn claim(&mut self, index: u32, transfer: StealthTransferStatement) -> Bucket {
            assert_eq!(self.status, Status::Awarded, "award not final");
            let award = self
                .awards
                .get(index as usize)
                .expect("unknown award")
                .clone();
            assert!(!award.claimed, "already claimed");
            assert_eq!(
                CallerContext::transaction_signer_public_key(),
                award.recipient,
                "recipient must sign"
            );
            let cell = self.cell.expect("escrow empty");
            require_exact_cell(&transfer, cell);
            assert_eq!(
                transfer.revealed_output_amount(),
                Amount::from(award.amount),
                "wrong payout amount"
            );
            assert!(award.amount <= self.remaining, "award exceeds remainder");
            let next = self.remaining - award.amount;
            if next == 0 {
                assert!(
                    transfer.stealth_outputs().is_empty(),
                    "no private output after final claim"
                );
            } else {
                assert_eq!(
                    transfer.stealth_outputs().len(),
                    1,
                    "one locked remainder required"
                );
                let output = &transfer.stealth_outputs()[0];
                assert_eq!(
                    output.auth,
                    SpendAuthorization::Script(self.lock_root()),
                    "remainder must stay locked"
                );
                assert_eq!(
                    output.output.minimum_value_promise, 1,
                    "constant minimum promise required"
                );
            }
            let new_cell = transfer
                .stealth_outputs()
                .first()
                .map(|o| o.output.commitment);
            let bucket = ResourceManager::get(TARI_TOKEN)
                .stealth_transfer(transfer)
                .expect("payout bucket required");
            self.awards[index as usize].claimed = true;
            self.cell = new_cell;
            self.remaining = next;
            if next == 0 {
                self.status = Status::Paid;
            }
            bucket
        }

        fn require_manager(&self) {
            assert_eq!(
                CallerContext::transaction_signer_public_key(),
                self.manager,
                "only council manager"
            );
        }

        fn reviewer_index(&self) -> usize {
            let caller = CallerContext::transaction_signer_public_key();
            self.reviewers
                .iter()
                .position(|key| *key == caller)
                .expect("not a reviewer")
        }

        fn award_quorum_met(&self) -> bool {
            self.award_approvals
                .iter()
                .filter(|approved| **approved)
                .count()
                >= self.review_quorum as usize
        }

        fn finalize_award_if_ready(&mut self) {
            if self.award_quorum_met() && self.tier != Tier::XL {
                self.status = Status::Awarded;
            }
        }
    }
}
