//! PROOF OF CONCEPT ONLY. No engine, transaction, recovery, or testnet qualification.
//! Forked from v1 so these experiments do not alter its published source.
use blake2::{Blake2b, Digest, digest::consts::U32};
use minicbor::{CborLen, Decode, Encode};
use std::collections::BTreeSet;
use tari_template_lib::{
    prelude::*,
    types::stealth::{SpendAuthorization, SpendCondition},
};

mod profiles;
pub use profiles::{
    CampaignProfile, ExampleConfig, FundingMode, example_terms, funding_roles,
    validate_pledge_role, validate_profile,
};

pub const PROTOCOL_VERSION: u32 = 100;
pub const MAX_PLEDGES: usize = 32;
pub const MAX_STAGES: usize = 8;
// Tari Crypto 0.23.2 RISTRETTO_NUMS_POINTS_COMPRESSED[0], the default value generator.
pub const VALUE_GENERATOR: [u8; 32] = [
    206, 56, 152, 65, 192, 200, 105, 138, 185, 91, 112, 36, 42, 238, 166, 72, 64, 177, 234, 197,
    246, 68, 183, 208, 8, 172, 5, 135, 207, 71, 29, 112,
];

#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct Stage {
    #[n(0)]
    pub leg: u32,
    #[n(1)]
    pub community: u64,
    #[n(2)]
    pub sponsor: u64,
    #[n(3)]
    pub recipient: RistrettoPublicKeyBytes,
    #[n(4)]
    pub reviewers: Vec<RistrettoPublicKeyBytes>,
    #[n(5)]
    pub quorum: u32,
    #[n(6)]
    pub deadline: u64,
    #[n(7)]
    pub upfront: bool,
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct Terms {
    #[n(0)]
    pub version: u32,
    #[n(1)]
    pub nonce: Hash32,
    #[n(2)]
    pub resource: ResourceAddress,
    #[n(3)]
    pub funding_deadline: u64,
    #[n(4)]
    pub sponsor_key: Option<RistrettoPublicKeyBytes>,
    #[n(5)]
    pub stages: Vec<Stage>,
    #[n(6)]
    pub mode: FundingMode,
    #[n(7)]
    pub profile: CampaignProfile,
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct Pledge {
    #[n(0)]
    pub owner: RistrettoPublicKeyBytes,
    #[n(1)]
    pub sponsor: bool,
    #[n(2)]
    pub cells: Vec<PedersenCommitmentBytes>,
    #[n(3)]
    pub refunded: bool,
}

/// Single-leaf MAST root copied from v1; these PoC templates need separate engine qualification.
pub fn escrow_leaf(component: ComponentAddress) -> SpendCondition {
    SpendCondition::access_rule(rule!(component(component)))
}

pub fn escrow_root(component: ComponentAddress) -> Hash32 {
    let domain = b"com.tari.ootle.engine.v0.ConditionLeaf";
    let mut digest = Blake2b::<U32>::new();
    digest.update((domain.len() as u64).to_le_bytes());
    digest.update(domain);
    digest.update(0u32.to_le_bytes());
    digest.update(borsh::to_vec(&escrow_leaf(component)).expect("condition encoding"));
    Hash32::from_array(digest.finalize().into())
}

pub fn validate_terms(terms: &Terms, now: u64) {
    validate_profile(terms, terms.profile);
    assert_eq!(terms.version, PROTOCOL_VERSION, "unsupported version");
    assert!(terms.funding_deadline > now, "funding deadline");
    assert!(
        !terms.stages.is_empty() && terms.stages.len() <= MAX_STAGES,
        "stage count"
    );
    assert_eq!(
        terms.sponsor_key.is_some(),
        terms.mode == FundingMode::Matched,
        "sponsor key does not match funding mode"
    );
    let mut previous = terms.funding_deadline;
    let mut total = 0u64;
    for (i, stage) in terms.stages.iter().enumerate() {
        assert!(stage.community > 0, "positive community budget required");
        match terms.mode {
            FundingMode::Matched => assert!(stage.sponsor > 0, "positive sponsor budget required"),
            FundingMode::CommunityOnly => {
                assert_eq!(stage.sponsor, 0, "community-only has no sponsor budget")
            }
        }
        total = total
            .checked_add(stage.community)
            .and_then(|v| v.checked_add(stage.sponsor))
            .expect("budget overflow");
        assert!(stage.deadline > previous, "ordered deadlines");
        previous = stage.deadline;
        assert!(!stage.upfront || i == 0, "upfront only first stage");
        let distinct = stage.reviewers.iter().collect::<BTreeSet<_>>();
        assert!(
            distinct.len() == stage.reviewers.len() && distinct.len() <= 5,
            "reviewer set"
        );
        if stage.upfront {
            assert!(
                stage.quorum == 0 && stage.reviewers.is_empty(),
                "upfront has no reviewers"
            );
        } else {
            assert!(
                stage.quorum > 0 && stage.quorum as usize <= distinct.len(),
                "quorum"
            );
        }
    }
}

/// Require the entire authorised value flow, not just one acceptable output.
pub fn validate_spend(expected: &[PedersenCommitmentBytes], transfer: &StealthTransferStatement) {
    assert!(!expected.is_empty(), "nothing eligible");
    assert!(
        transfer.revealed_input_amount().is_zero() && transfer.revealed_output_amount().is_zero(),
        "public flow forbidden"
    );
    let actual = transfer
        .stealth_inputs()
        .iter()
        .map(|i| i.commitment)
        .collect::<BTreeSet<_>>();
    let required = expected.iter().copied().collect::<BTreeSet<_>>();
    assert!(
        actual.len() == transfer.stealth_inputs().len() && required.len() == expected.len(),
        "duplicate inputs"
    );
    assert_eq!(actual, required, "exact input set required");
    assert_eq!(
        transfer.stealth_outputs().len(),
        1,
        "one private output required"
    );
    assert!(
        matches!(
            transfer.stealth_outputs()[0].auth,
            SpendAuthorization::Key(_)
        ),
        "key output required"
    );
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct Campaign {
    #[n(0)]
    terms: Terms,
    #[n(1)]
    pledges: Vec<Pledge>,
    #[n(2)]
    selected: Vec<u32>,
    #[n(3)]
    active: bool,
    #[n(4)]
    next_stage: u32,
    // Per-stage, per-reviewer evidence. No opaque coordinator authority.
    #[n(5)]
    votes: Vec<Vec<Option<Hash32>>>,
}

impl Campaign {
    pub fn new(terms: Terms, profile: CampaignProfile) -> Self {
        validate_profile(&terms, profile);
        validate_terms(&terms, Consensus::current_epoch());
        let votes = terms
            .stages
            .iter()
            .map(|s| vec![None; s.reviewers.len()])
            .collect();
        Self {
            terms,
            pledges: vec![],
            selected: vec![],
            active: false,
            next_stage: 0,
            votes,
        }
    }

    pub fn terms(&self) -> Terms {
        self.terms.clone()
    }
    pub fn pledges(&self) -> Vec<Pledge> {
        self.pledges.clone()
    }
    pub fn status(&self) -> (bool, u32, Vec<u32>) {
        (self.active, self.next_stage, self.selected.clone())
    }
    pub fn lock_root(&self) -> Hash32 {
        escrow_root(CallerContext::current_component_address())
    }

    /// Execute deposit and register its confidential cells in the same ledger operation.
    pub fn pledge(&mut self, sponsor: bool, transfer: StealthTransferStatement) -> u32 {
        assert!(
            !self.active && Consensus::current_epoch() < self.terms.funding_deadline,
            "funding closed"
        );
        assert!(self.pledges.len() < MAX_PLEDGES, "pledge cap");
        let owner = CallerContext::transaction_signer_public_key();
        validate_pledge_role(&self.terms, &owner, sponsor);
        if sponsor {
            assert!(
                !self.pledges.iter().any(|p| p.sponsor),
                "one sponsor pledge"
            );
        }
        assert!(
            transfer.revealed_input_amount().is_zero()
                && transfer.revealed_output_amount().is_zero(),
            "private deposit required"
        );
        assert_eq!(
            transfer.stealth_outputs().len(),
            self.terms.stages.len(),
            "one cell per stage"
        );
        let known = self
            .pledges
            .iter()
            .flat_map(|p| p.cells.iter())
            .copied()
            .collect::<BTreeSet<_>>();
        assert!(
            transfer
                .stealth_inputs()
                .iter()
                .all(|i| !known.contains(&i.commitment)),
            "escrow recycling forbidden"
        );
        let root = self.lock_root();
        let mut cells = Vec::new();
        for out in transfer.stealth_outputs() {
            assert_eq!(
                out.auth,
                SpendAuthorization::Script(root),
                "pure component lock required"
            );
            assert_eq!(
                out.output.minimum_value_promise, 1,
                "constant minimum promise required"
            );
            assert!(
                !known.contains(&out.output.commitment) && !cells.contains(&out.output.commitment),
                "duplicate commitment"
            );
            cells.push(out.output.commitment);
        }
        let result = ResourceManager::get(self.terms.resource).stealth_transfer(transfer);
        assert!(result.is_none(), "unexpected public deposit");
        let id = self.pledges.len() as u32;
        self.pledges.push(Pledge {
            owner,
            sponsor,
            cells,
            refunded: false,
        });
        id
    }

    /// All role/stage sums are checked; there is no caller-selected list of bundle legs.
    pub fn activate(&mut self, selected: Vec<u32>, aggregate_masks: Vec<Scalar32Bytes>) {
        assert!(
            !self.active && Consensus::current_epoch() < self.terms.funding_deadline,
            "activation closed"
        );
        assert!(
            !selected.is_empty() && selected.len() <= MAX_PLEDGES,
            "selection size"
        );
        assert_eq!(
            selected.iter().collect::<BTreeSet<_>>().len(),
            selected.len(),
            "duplicate pledge"
        );
        let roles = funding_roles(self.terms.mode);
        assert_eq!(
            aggregate_masks.len(),
            self.terms.stages.len() * roles,
            "all role/stage openings required"
        );
        for id in &selected {
            let p = self.pledges.get(*id as usize).expect("unknown pledge");
            assert!(!p.refunded, "refunded pledge");
        }
        for (j, stage) in self.terms.stages.iter().enumerate() {
            for role in 0..roles {
                let mut sum = RistrettoPublicKeyBytes::from([0u8; 32]);
                let mut count = 0;
                for id in &selected {
                    let p = &self.pledges[*id as usize];
                    if p.sponsor == (role == 1) {
                        sum = intrinsics::ristretto_add(
                            &sum,
                            &RistrettoPublicKeyBytes::from(p.cells[j].into_array()),
                        );
                        count += 1;
                    }
                }
                assert!(count > 0, "missing funding role");
                let value = if role == 0 {
                    stage.community
                } else {
                    stage.sponsor
                };
                let mut scalar = [0u8; 32];
                scalar[..8].copy_from_slice(&value.to_le_bytes());
                let expected = intrinsics::ristretto_add(
                    &intrinsics::ristretto_mul_base(&aggregate_masks[j * roles + role]),
                    &intrinsics::ristretto_mul(
                        &RistrettoPublicKeyBytes::from(VALUE_GENERATOR),
                        &Scalar32Bytes::from(scalar),
                    ),
                );
                assert_eq!(sum, expected, "exact confidential budget not met");
            }
        }
        self.selected = selected;
        self.active = true;
    }

    pub fn approve(&mut self, stage: u32, evidence: Hash32) {
        self.require_current(stage);
        let term = &self.terms.stages[stage as usize];
        assert!(!term.upfront, "upfront needs no approval");
        assert_ne!(evidence, Hash32::from_array([0; 32]), "empty evidence");
        let key = CallerContext::transaction_signer_public_key();
        let reviewer = term
            .reviewers
            .iter()
            .position(|k| *k == key)
            .expect("not a reviewer");
        let vote = &mut self.votes[stage as usize][reviewer];
        assert!(vote.is_none(), "already voted");
        *vote = Some(evidence);
    }

    pub fn release(&mut self, stage: u32, evidence: Hash32, transfer: StealthTransferStatement) {
        self.require_current(stage);
        let term = &self.terms.stages[stage as usize];
        assert_eq!(
            CallerContext::transaction_signer_public_key(),
            term.recipient,
            "recipient must sign payout"
        );
        if !term.upfront {
            let votes = self.votes[stage as usize]
                .iter()
                .filter(|v| **v == Some(evidence))
                .count();
            assert!(votes >= term.quorum as usize, "review quorum not met");
        }
        let expected = self
            .selected
            .iter()
            .map(|id| self.pledges[*id as usize].cells[stage as usize])
            .collect::<Vec<_>>();
        validate_spend(&expected, &transfer);
        assert!(
            ResourceManager::get(self.terms.resource)
                .stealth_transfer(transfer)
                .is_none(),
            "public payout"
        );
        self.next_stage += 1;
    }

    pub fn refund(&mut self, id: u32, transfer: StealthTransferStatement) {
        let p = self.pledges.get(id as usize).expect("unknown pledge");
        assert!(!p.refunded, "already refunded");
        assert_eq!(
            CallerContext::transaction_signer_public_key(),
            p.owner,
            "refund owner must sign"
        );
        let selected = self.active && self.selected.contains(&id);
        let now = Consensus::current_epoch();
        if !self.active {
            assert!(now >= self.terms.funding_deadline, "funding still open");
        } else if selected {
            let stage = self
                .terms
                .stages
                .get(self.next_stage as usize)
                .expect("nothing remains");
            assert!(now >= stage.deadline, "delivery still open");
        }
        let start = if selected {
            self.next_stage as usize
        } else {
            0
        };
        validate_spend(&p.cells[start..], &transfer);
        assert!(
            ResourceManager::get(self.terms.resource)
                .stealth_transfer(transfer)
                .is_none(),
            "public refund"
        );
        self.pledges[id as usize].refunded = true;
    }

    fn require_current(&self, stage: u32) {
        assert!(
            self.active && stage == self.next_stage,
            "wrong active stage"
        );
        let term = self.terms.stages.get(stage as usize).expect("complete");
        assert!(
            Consensus::current_epoch() < term.deadline,
            "delivery deadline passed"
        );
    }
}
