fn main() {
    tari_ootle_template_build::TemplateMetadataBuilder::new()
        .description("Experimental Tari-style issue bounty escrow. No GitHub verification or deployment claim.")
        .enable_json_output()
        .build()
        .expect("template metadata");
}
