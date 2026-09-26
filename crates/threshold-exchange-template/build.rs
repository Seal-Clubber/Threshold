fn main() {
    tari_ootle_template_build::TemplateMetadataBuilder::new()
        .description("PROOF OF CONCEPT ONLY: Exchange listing and integration. Not deployed or end-to-end tested.")
        .enable_json_output().build().expect("template metadata");
}
