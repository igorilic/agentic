use agentic_core::Db;
use agentic_core::db::pipeline_presets::{PipelinePreset, PipelinePresetRepo};

fn setup() -> (Db, PipelinePresetRepo) {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    let repo = PipelinePresetRepo::new(&db);
    (db, repo)
}

fn default_agents() -> Vec<String> {
    vec![
        "architect".to_string(),
        "tdd-developer".to_string(),
        "qa".to_string(),
        "reviewer".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// Schema test
// ---------------------------------------------------------------------------

#[test]
fn migration_0010_creates_pipeline_presets_table() {
    let (db, _repo) = setup();
    let conn = db.conn().expect("conn");
    let mut stmt = conn
        .prepare("PRAGMA table_info(pipeline_presets)")
        .expect("prepare pragma");

    struct ColInfo {
        name: String,
        col_type: String,
        not_null: bool,
    }

    let cols: Vec<ColInfo> = stmt
        .query_map([], |r| {
            Ok(ColInfo {
                name: r.get::<_, String>(1)?,
                col_type: r.get::<_, String>(2)?,
                not_null: r.get::<_, i32>(3)? != 0,
            })
        })
        .expect("query_map")
        .map(|r| r.expect("col"))
        .collect();

    let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    for expected in &["id", "name", "agents", "created_at", "updated_at"] {
        assert!(
            names.contains(expected),
            "expected column {expected} in pipeline_presets; found: {names:?}"
        );
    }

    // Verify NOT NULL constraints
    for col in &cols {
        if matches!(
            col.name.as_str(),
            "id" | "name" | "agents" | "created_at" | "updated_at"
        ) {
            assert!(
                col.not_null || col.name == "id", // id is PK, implicitly not null
                "column {} should be NOT NULL",
                col.name
            );
        }
    }

    // Verify TEXT types for id/name/agents
    for col in &cols {
        if matches!(col.name.as_str(), "id" | "name" | "agents") {
            assert_eq!(
                col.col_type.to_uppercase(),
                "TEXT",
                "column {} should be TEXT, got {}",
                col.name,
                col.col_type
            );
        }
    }

    // Verify INTEGER types for timestamps
    for col in &cols {
        if matches!(col.name.as_str(), "created_at" | "updated_at") {
            assert_eq!(
                col.col_type.to_uppercase(),
                "INTEGER",
                "column {} should be INTEGER, got {}",
                col.name,
                col.col_type
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Create tests
// ---------------------------------------------------------------------------

#[test]
fn create_returns_a_preset_with_fresh_ulid_and_timestamps() {
    let (_db, repo) = setup();

    let agents = default_agents();
    let preset = repo.create("default", &agents).expect("create");

    assert_eq!(
        preset.id.len(),
        26,
        "ULID should be 26 chars: {}",
        preset.id
    );
    assert_eq!(preset.name, "default");
    assert_eq!(preset.agents, agents);
    assert!(preset.created_at > 0, "created_at should be non-zero");
    assert_eq!(
        preset.created_at, preset.updated_at,
        "created_at and updated_at should be equal on create"
    );
}

#[test]
fn create_persists_to_db() {
    let (_db, repo) = setup();
    let preset = repo.create("default", &default_agents()).expect("create");

    let list = repo.list().expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, preset.id);
    assert_eq!(list[0].name, preset.name);
    assert_eq!(list[0].agents, preset.agents);
    assert_eq!(list[0].created_at, preset.created_at);
    assert_eq!(list[0].updated_at, preset.updated_at);
}

#[test]
fn create_with_duplicate_name_errors() {
    let (_db, repo) = setup();
    repo.create("my-preset", &default_agents())
        .expect("first create");
    let result = repo.create("my-preset", &default_agents());
    assert!(result.is_err(), "expected error for duplicate name");
}

#[test]
fn create_with_empty_name_after_trim_errors() {
    let (_db, repo) = setup();
    let result = repo.create("   ", &default_agents());
    assert!(result.is_err(), "expected error for blank name");
}

#[test]
fn create_with_overlong_name_errors() {
    let (_db, repo) = setup();
    let long_name = "a".repeat(65);
    let result = repo.create(&long_name, &default_agents());
    assert!(result.is_err(), "expected error for 65-char name");
}

#[test]
fn create_with_exactly_64_char_name_succeeds() {
    let (_db, repo) = setup();
    let name = "a".repeat(64);
    let result = repo.create(&name, &default_agents());
    assert!(
        result.is_ok(),
        "64-char name should be accepted: {:?}",
        result
    );
}

#[test]
fn create_with_empty_agents_errors() {
    let (_db, repo) = setup();
    let result = repo.create("my-preset", &[]);
    assert!(result.is_err(), "expected error for empty agents list");
}

// ---------------------------------------------------------------------------
// Update tests
// ---------------------------------------------------------------------------

#[test]
fn update_changes_name_and_agents_and_bumps_updated_at() {
    let (_db, repo) = setup();
    let preset = repo.create("original", &default_agents()).expect("create");
    let original_created_at = preset.created_at;

    // Sleep briefly to ensure updated_at > created_at
    std::thread::sleep(std::time::Duration::from_millis(2));

    let new_agents = vec!["qa".to_string(), "reviewer".to_string()];
    let updated = repo
        .update(&preset.id, "renamed", &new_agents)
        .expect("update");

    assert_eq!(updated.id, preset.id);
    assert_eq!(updated.name, "renamed");
    assert_eq!(updated.agents, new_agents);
    assert_eq!(
        updated.created_at, original_created_at,
        "created_at must not change"
    );
    assert!(
        updated.updated_at > original_created_at,
        "updated_at ({}) must be > created_at ({})",
        updated.updated_at,
        original_created_at
    );
}

#[test]
fn update_with_unknown_id_errors() {
    let (_db, repo) = setup();
    let result = repo.update("01ABCFAKE00000000000000000", "name", &default_agents());
    assert!(result.is_err(), "expected error for unknown id");
}

#[test]
fn update_to_duplicate_name_errors() {
    let (_db, repo) = setup();
    let a = repo
        .create("preset-a", &default_agents())
        .expect("create a");
    repo.create("preset-b", &default_agents())
        .expect("create b");

    // Try to rename a to b's name
    let result = repo.update(&a.id, "preset-b", &default_agents());
    assert!(
        result.is_err(),
        "expected error for duplicate name on update"
    );
}

// ---------------------------------------------------------------------------
// Delete tests
// ---------------------------------------------------------------------------

#[test]
fn delete_removes_row() {
    let (_db, repo) = setup();
    let preset = repo.create("to-delete", &default_agents()).expect("create");

    repo.delete(&preset.id).expect("delete");

    let list = repo.list().expect("list");
    assert!(list.is_empty(), "list should be empty after delete");
}

#[test]
fn delete_unknown_id_errors() {
    let (_db, repo) = setup();
    let result = repo.delete("01ABCFAKE00000000000000000");
    assert!(result.is_err(), "expected error for deleting unknown id");
}

// ---------------------------------------------------------------------------
// Get tests
// ---------------------------------------------------------------------------

#[test]
fn get_by_id_returns_some_when_present() {
    let (_db, repo) = setup();
    let created = repo.create("my-preset", &default_agents()).expect("create");

    let found = repo.get_by_id(&created.id).expect("get_by_id");
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.name, created.name);
    assert_eq!(found.agents, created.agents);
}

#[test]
fn get_by_id_returns_none_when_absent() {
    let (_db, repo) = setup();
    let result = repo.get_by_id("missing-id").expect("get_by_id");
    assert!(result.is_none());
}

#[test]
fn get_by_name_returns_some_when_present() {
    let (_db, repo) = setup();
    repo.create("known-preset", &default_agents())
        .expect("create");

    let found = repo.get_by_name("known-preset").expect("get_by_name");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "known-preset");
}

#[test]
fn get_by_name_returns_none_when_absent() {
    let (_db, repo) = setup();
    let result = repo.get_by_name("no-such-preset").expect("get_by_name");
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Order / round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn list_orders_by_name_asc() {
    let (_db, repo) = setup();
    repo.create("zeta", &default_agents()).expect("create zeta");
    repo.create("alpha", &default_agents())
        .expect("create alpha");
    repo.create("mu", &default_agents()).expect("create mu");

    let list = repo.list().expect("list");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].name, "alpha");
    assert_eq!(list[1].name, "mu");
    assert_eq!(list[2].name, "zeta");
}

#[test]
fn agents_round_trip_preserves_order() {
    let (_db, repo) = setup();
    // Note: [a, b, c, b] — duplicate b is intentional
    let agents = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "b".to_string(),
    ];
    repo.create("ordered", &agents).expect("create");

    let list = repo.list().expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].agents, agents);
}

#[test]
fn name_is_trimmed_on_create_and_update() {
    let (_db, repo) = setup();

    // Create with whitespace — stored name should be trimmed
    let preset = repo.create("  foo  ", &default_agents()).expect("create");
    assert_eq!(preset.name, "foo");

    // Update with whitespace — stored name should also be trimmed
    let updated = repo
        .update(&preset.id, "\t bar \n", &default_agents())
        .expect("update");
    assert_eq!(updated.name, "bar");
}

// ---------------------------------------------------------------------------
// PipelinePreset struct implements expected traits
// ---------------------------------------------------------------------------

#[test]
fn pipeline_preset_implements_debug_clone_partialeq() {
    let p = PipelinePreset {
        id: "01ABCDEF0123456789012345".to_string(),
        name: "test".to_string(),
        agents: vec!["a".to_string()],
        created_at: 1000,
        updated_at: 1000,
    };
    let cloned = p.clone();
    assert_eq!(p, cloned);
    let _ = format!("{p:?}"); // debug
}
