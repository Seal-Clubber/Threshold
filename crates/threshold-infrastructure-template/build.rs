fn main() {
    tari_ootle_template_build::TemplateMetadataBuilder::new()
        .description("PROOF OF CONCEPT ONLY: Shared infrastructure and maintenance. Not deployed or end-to-end tested.")
        .enable_json_output().build().expect("template metadata");
}
