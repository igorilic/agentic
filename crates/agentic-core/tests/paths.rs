use agentic_core::paths::Paths;

#[test]
fn for_tests_roots_all_paths_under_base() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Paths::for_tests(tmp.path());
    assert!(
        p.config_dir().starts_with(tmp.path()),
        "config_dir not under tmp: {:?}",
        p.config_dir()
    );
    assert!(
        p.data_dir().starts_with(tmp.path()),
        "data_dir not under tmp: {:?}",
        p.data_dir()
    );
    assert!(
        p.log_dir().starts_with(tmp.path()),
        "log_dir not under tmp: {:?}",
        p.log_dir()
    );
}

#[test]
fn config_file_ends_with_settings_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Paths::for_tests(tmp.path());
    assert_eq!(p.config_file().file_name().unwrap(), "settings.toml");
}

#[test]
fn db_file_ends_with_state_db() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Paths::for_tests(tmp.path());
    assert_eq!(p.db_file().file_name().unwrap(), "state.db");
}

#[test]
fn ensure_dirs_creates_missing_parents_and_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Paths::for_tests(tmp.path());
    // First call: dirs do not exist yet.
    p.ensure_dirs().expect("ensure_dirs first call");
    assert!(p.config_dir().is_dir());
    assert!(p.data_dir().is_dir());
    assert!(p.log_dir().is_dir());
    // Second call: must not error (idempotent).
    p.ensure_dirs().expect("ensure_dirs is idempotent");
}
