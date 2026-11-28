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

#[test]
fn operation_summaries_are_short_titles() {
    const METHODS: [&str; 8] = [
        "get", "put", "post", "delete", "options", "head", "patch", "trace",
    ];
    let spec: serde_json::Value =
        serde_json::from_str(&oxistream_server::api::spec_json()).unwrap();
    let mut bad = Vec::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            if !METHODS.contains(&method.as_str()) {
                continue;
            }
            let summary = operation["summary"].as_str().unwrap_or_default();
            if summary.is_empty()
                || summary.contains('\n')
                || summary.chars().count() > 60
                || summary.ends_with('.')
            {
                bad.push(format!("{} {path}: {summary:?}", method.to_uppercase()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "operation summaries must be a single short title (at most 60 characters, no \
         trailing period); put details in a second doc comment paragraph:\n{}",
        bad.join("\n")
    );
}
