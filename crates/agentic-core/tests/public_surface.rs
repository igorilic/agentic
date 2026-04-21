#[test]
fn version_equals_cargo_pkg_version() {
    assert_eq!(agentic_core::VERSION, env!("CARGO_PKG_VERSION"));
}
