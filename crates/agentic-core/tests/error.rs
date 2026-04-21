use agentic_core::error::CoreError;

#[test]
fn core_error_is_send_sync_static() {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<CoreError>();
}

#[test]
fn io_variant_displays_with_source_chain() {
    let underlying = std::io::Error::new(std::io::ErrorKind::NotFound, "disk gone");
    let e = CoreError::Io(underlying);
    let rendered = format!("{e}");
    assert!(
        rendered.contains("io error") && rendered.contains("disk gone"),
        "expected message to include both outer tag and inner cause, got: {rendered}"
    );
    // Preserves source chain
    let src = std::error::Error::source(&e).expect("Io variant exposes source");
    assert_eq!(src.to_string(), "disk gone");
}

#[test]
fn from_io_error_conversion_works() {
    let underlying = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
    let e: CoreError = underlying.into();
    assert!(matches!(e, CoreError::Io(_)));
}

#[test]
fn result_alias_works() {
    fn ok() -> agentic_core::error::Result<u8> {
        Ok(42)
    }
    assert_eq!(ok().unwrap(), 42);
}
