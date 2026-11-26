use std::path::Path;

const SPEC_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../openapi.json");

#[test]
fn committed_openapi_spec_is_up_to_date() {
    let generated = oxistream_server::api::spec_json();

    if std::env::var_os("OXISTREAM_UPDATE_OPENAPI").is_some() {
        std::fs::write(SPEC_PATH, &generated).unwrap();
        return;
    }

    let committed = std::fs::read_to_string(Path::new(SPEC_PATH)).unwrap_or_default();
    assert!(
        committed == generated,
        "backend/openapi.json is out of date; regenerate it with \
         `cargo run -p oxistream-server -- openapi > openapi.json` \
         or `OXISTREAM_UPDATE_OPENAPI=1 cargo test --test openapi`"
    );
}
