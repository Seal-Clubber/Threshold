fn main() {
    tari_ootle_template_build::TemplateMetadataBuilder::new()
        .description("Threshold: confidential milestone tranches and atomic matched bundles")
        .enable_json_output().build().expect("template metadata");
}
