//! Typed construction helpers. These do not submit or validate a transaction.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, CborLen)]
#[cbor(index_only)]
pub enum FundingMode {
    #[n(0)]
    Matched,
    #[n(1)]
    CommunityOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, CborLen)]
#[cbor(index_only)]
pub enum CampaignProfile {
    #[n(0)]
    Exchange,
    #[n(1)]
    Audit,
    #[n(2)]
    Infrastructure,
    #[n(3)]
    Grant,
    #[n(4)]
    Community,
}

/// Caller supplies real public keys and future network epochs; no test secrets are embedded.
/// One recipient per stage, with the same reviewer set for every reviewed example stage.
#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub struct ExampleConfig {
    #[n(0)]
    pub nonce: Hash32,
    #[n(1)]
    pub resource: ResourceAddress,
    #[n(2)]
    pub funding_deadline: u64,
    #[n(3)]
    pub stage_spacing: u64,
    #[n(4)]
    pub sponsor_key: Option<RistrettoPublicKeyBytes>,
    #[n(5)]
    pub recipients: Vec<RistrettoPublicKeyBytes>,
    #[n(6)]
    pub reviewers: Vec<RistrettoPublicKeyBytes>,
}

pub fn funding_roles(mode: FundingMode) -> usize {
    match mode {
        FundingMode::Matched => 2,
        FundingMode::CommunityOnly => 1,
    }
}

pub fn validate_profile(terms: &Terms, expected: CampaignProfile) {
    assert_eq!(terms.profile, expected, "wrong campaign profile");
    let mode = if expected == CampaignProfile::Community {
        FundingMode::CommunityOnly
    } else {
        FundingMode::Matched
    };
    assert_eq!(terms.mode, mode, "wrong funding mode for this template");
    if expected == CampaignProfile::Audit {
        assert!(
            terms.stages.len() >= 2,
            "audit needs development and audit stages"
        );
        assert_ne!(
            terms.stages[0].recipient,
            terms.stages.last().unwrap().recipient,
            "auditor needs a separate recipient key"
        );
    }
}

pub fn validate_pledge_role(terms: &Terms, owner: &RistrettoPublicKeyBytes, sponsor: bool) {
    match terms.mode {
        FundingMode::Matched => {
            let key = terms.sponsor_key.as_ref().expect("missing sponsor key");
            assert_eq!(sponsor, owner == key, "sponsor role mismatch");
        }
        FundingMode::CommunityOnly => assert!(!sponsor, "community-only rejects sponsor pledges"),
    }
}

/// Fixed, illustrative economics; call the template's `new` with custom Terms for other schedules.
/// Amounts below are integer micro-units of the configured resource, illustrated as tTARI.
pub fn example_terms(profile: CampaignProfile, config: ExampleConfig) -> Terms {
    let (totals, share, quorum): (&[u64], u64, u32) = match profile {
        CampaignProfile::Exchange => (&[100, 400, 500], 60, 2),
        CampaignProfile::Audit => (&[50, 250, 200], 70, 2),
        CampaignProfile::Infrastructure => (&[12, 72, 72, 84], 75, 2),
        CampaignProfile::Grant => (&[20, 90, 90], 50, 3),
        CampaignProfile::Community => (&[10, 90], 100, 2),
    };
    assert_eq!(
        config.recipients.len(),
        totals.len(),
        "one recipient key per example stage"
    );
    assert!(config.stage_spacing > 0, "positive stage spacing required");
    let stages = totals
        .iter()
        .enumerate()
        .map(|(index, total)| {
            let amount = total
                .checked_mul(1_000_000)
                .expect("example amount overflow");
            let community = amount.checked_mul(share).expect("example share overflow") / 100;
            let offset = config
                .stage_spacing
                .checked_mul(index as u64 + 1)
                .expect("deadline overflow");
            Stage {
                leg: index as u32,
                community,
                sponsor: amount - community,
                recipient: config.recipients[index],
                reviewers: if index == 0 {
                    vec![]
                } else {
                    config.reviewers.clone()
                },
                quorum: if index == 0 { 0 } else { quorum },
                deadline: config
                    .funding_deadline
                    .checked_add(offset)
                    .expect("deadline overflow"),
                upfront: index == 0,
            }
        })
        .collect();
    let terms = Terms {
        version: PROTOCOL_VERSION,
        nonce: config.nonce,
        resource: config.resource,
        funding_deadline: config.funding_deadline,
        sponsor_key: config.sponsor_key,
        stages,
        mode: if profile == CampaignProfile::Community {
            FundingMode::CommunityOnly
        } else {
            FundingMode::Matched
        },
        profile,
    };
    // Only static term checks here. Deployment checks against the actual consensus epoch again.
    validate_terms(&terms, 0);
    terms
}
