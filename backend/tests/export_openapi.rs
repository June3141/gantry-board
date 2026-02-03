use utoipa::OpenApi;

#[test]
fn export_openapi_spec() {
    let spec = gantry_board::openapi::ApiDoc::openapi()
        .to_json()
        .expect("failed to serialize OpenAPI spec");

    let out_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../openapi.json");
    std::fs::write(&out_path, spec).expect("failed to write openapi.json");

    println!("OpenAPI spec exported to {}", out_path.display());
}
