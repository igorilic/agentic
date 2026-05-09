#![cfg(test)]

use agentic_core::Db;
use agentic_tauri::commands::pipeline_presets::{
    delete_pipeline_preset, list_pipeline_presets, save_pipeline_preset, update_pipeline_preset,
};
use tauri::Manager;

fn build_app(db: &Db) -> tauri::App<tauri::test::MockRuntime> {
    use tauri::test::{mock_builder, mock_context, noop_assets};
    mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::pipeline_presets::list_pipeline_presets,
            agentic_tauri::commands::pipeline_presets::save_pipeline_preset,
            agentic_tauri::commands::pipeline_presets::update_pipeline_preset,
            agentic_tauri::commands::pipeline_presets::delete_pipeline_preset,
        ])
        .manage(db.clone())
        .build(mock_context(noop_assets()))
        .expect("build mock app")
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_returns_empty_when_no_presets() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let rows = list_pipeline_presets(app.state::<Db>())
        .await
        .expect("list");
    assert!(rows.is_empty());
}

// ---------------------------------------------------------------------------
// save (create)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn save_inserts_new_preset_and_returns_it() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let preset = save_pipeline_preset(
        app.state::<Db>(),
        "default".to_string(),
        vec!["architect".to_string()],
    )
    .await
    .expect("save");

    assert!(!preset.id.is_empty());
    assert_eq!(preset.name, "default");
    assert_eq!(preset.agents, vec!["architect".to_string()]);

    // Subsequent list returns 1 row.
    let rows = list_pipeline_presets(app.state::<Db>())
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, preset.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn save_with_duplicate_name_errors() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    save_pipeline_preset(
        app.state::<Db>(),
        "duplicate".to_string(),
        vec!["architect".to_string()],
    )
    .await
    .expect("first save");

    let err = save_pipeline_preset(
        app.state::<Db>(),
        "duplicate".to_string(),
        vec!["tdd-developer".to_string()],
    )
    .await
    .expect_err("duplicate name must error");

    assert!(!err.is_empty(), "error string must not be empty: {err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn save_with_empty_name_errors() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let err = save_pipeline_preset(
        app.state::<Db>(),
        "   ".to_string(),
        vec!["architect".to_string()],
    )
    .await
    .expect_err("whitespace-only name must error");

    assert!(!err.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn save_with_empty_agents_errors() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let err = save_pipeline_preset(app.state::<Db>(), "foo".to_string(), vec![])
        .await
        .expect_err("empty agents must error");

    assert!(!err.is_empty());
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn update_changes_name_and_agents() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let created = save_pipeline_preset(
        app.state::<Db>(),
        "original".to_string(),
        vec!["architect".to_string()],
    )
    .await
    .expect("save");

    let updated = update_pipeline_preset(
        app.state::<Db>(),
        created.id.clone(),
        "renamed".to_string(),
        vec!["tdd-developer".to_string(), "qa".to_string()],
    )
    .await
    .expect("update");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "renamed");
    assert_eq!(
        updated.agents,
        vec!["tdd-developer".to_string(), "qa".to_string()]
    );

    // List confirms the change.
    let rows = list_pipeline_presets(app.state::<Db>())
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "renamed");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_with_unknown_id_errors() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let err = update_pipeline_preset(
        app.state::<Db>(),
        "01ABCFAKEUNKNOWNID000000000".to_string(),
        "any name".to_string(),
        vec!["architect".to_string()],
    )
    .await
    .expect_err("unknown id must error");

    assert!(!err.is_empty());
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn delete_removes_row() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let created = save_pipeline_preset(
        app.state::<Db>(),
        "to-delete".to_string(),
        vec!["architect".to_string()],
    )
    .await
    .expect("save");

    delete_pipeline_preset(app.state::<Db>(), created.id.clone())
        .await
        .expect("delete");

    let rows = list_pipeline_presets(app.state::<Db>())
        .await
        .expect("list");
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_unknown_id_errors() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    let err = delete_pipeline_preset(
        app.state::<Db>(),
        "01ABCFAKEUNKNOWNID000000000".to_string(),
    )
    .await
    .expect_err("unknown id must error");

    assert!(!err.is_empty());
}

// ---------------------------------------------------------------------------
// ordering
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_orders_by_name_asc() {
    let db = Db::open_in_memory().unwrap();
    let app = build_app(&db);

    // Insert in non-alphabetical order.
    for name in &["zeta", "alpha", "mu"] {
        save_pipeline_preset(
            app.state::<Db>(),
            name.to_string(),
            vec!["architect".to_string()],
        )
        .await
        .expect("save");
    }

    let rows = list_pipeline_presets(app.state::<Db>())
        .await
        .expect("list");

    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "mu", "zeta"]);
}
