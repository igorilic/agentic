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

#[test]
fn result_alias_works_at_crate_root() {
    fn returns_ok() -> agentic_core::Result<()> {
        Ok(())
    }
    assert!(returns_ok().is_ok());
}

// #4 — CoreError::Other(anyhow) exposes .source() when the anyhow error wraps
// a typed source via context(). A bare anyhow!("msg") has no inner source.
#[test]
fn other_variant_preserves_source_chain() {
    use std::error::Error;
    // Wrap a real typed error with context so there is a source chain.
    let io_err = std::io::Error::other("root cause");
    let with_context = anyhow::Error::new(io_err).context("outer context");
    let err: CoreError = with_context.into();
    assert!(
        err.source().is_some(),
        "Other variant should expose source when anyhow wraps a typed error; got: {err}"
    );
    let chain: Vec<String> = std::iter::successors(err.source(), |e| (*e).source())
        .map(|e| e.to_string())
        .collect();
    assert!(
        chain.iter().any(|s| s.contains("root cause")),
        "source chain should contain 'root cause'; chain: {chain:?}"
    );
}
