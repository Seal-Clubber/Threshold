//! PROOF OF CONCEPT ONLY: Exchange listing and integration.
//! Compile and term checks only; no engine, transaction, or recovery qualification.
use tari_template_lib::prelude::*;
use threshold_poc_core::{Campaign, CampaignProfile, ExampleConfig, Pledge, Terms, example_terms};

#[template]
mod exchange_funding {
    use super::*;

    pub struct ExchangeFunding {
        campaign: Campaign,
    }

    impl ExchangeFunding {
        /// Supply custom campaign terms. Sponsor mode and profile are enforced by this template.
        pub fn new(terms: Terms) -> Component<Self> {
            let campaign = Campaign::new(terms, CampaignProfile::Exchange);
            Component::new(Self { campaign })
                .with_owner_rule(OwnerRule::None)
                .with_access_rules(AccessRules::allow_all())
                .create()
        }

        /// Example terms using caller-supplied public keys, resource and deadlines.
        pub fn example(config: ExampleConfig) -> Terms {
            example_terms(CampaignProfile::Exchange, config)
        }

        pub fn new_example(config: ExampleConfig) -> Component<Self> {
            Self::new(Self::example(config))
        }

        pub fn terms(&self) -> Terms {
            self.campaign.terms()
        }
        pub fn pledges(&self) -> Vec<Pledge> {
            self.campaign.pledges()
        }
        pub fn status(&self) -> (bool, u32, Vec<u32>) {
            self.campaign.status()
        }
        pub fn lock_root(&self) -> Hash32 {
            self.campaign.lock_root()
        }
        pub fn pledge(&mut self, sponsor: bool, transfer: StealthTransferStatement) -> u32 {
            self.campaign.pledge(sponsor, transfer)
        }
        pub fn activate(&mut self, selected: Vec<u32>, aggregate_masks: Vec<Scalar32Bytes>) {
            self.campaign.activate(selected, aggregate_masks);
        }
        pub fn approve(&mut self, stage: u32, evidence: Hash32) {
            self.campaign.approve(stage, evidence);
        }
        pub fn release(
            &mut self,
            stage: u32,
            evidence: Hash32,
            transfer: StealthTransferStatement,
        ) {
            self.campaign.release(stage, evidence, transfer);
        }
        pub fn refund(&mut self, id: u32, transfer: StealthTransferStatement) {
            self.campaign.refund(id, transfer);
        }
    }
}
