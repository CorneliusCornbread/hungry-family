use std::fs;

#[test]
fn vendored_htmx_matches_manifest() {
    let manifest = fs::read_to_string("static/vendor/htmx.version").expect("manifest readable");
    let version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("HTMX_VERSION="))
        .expect("HTMX_VERSION entry")
        .trim();

    assert!(!version.is_empty(), "HTMX_VERSION is empty");
    assert_eq!(version, version.trim_start_matches('v'), "strip the v prefix from HTMX_VERSION");

    let js = fs::read_to_string("static/vendor/htmx.min.js").expect("vendored htmx readable");
    let expected = format!("version:\"{version}\"");
    assert!(
        js.contains(&expected),
        "vendored htmx.min.js does not declare {expected}; re-run scripts/vendor-htmx.sh"
    );
}
