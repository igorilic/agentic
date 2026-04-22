use agentic_core::{CoreError, Db, Paths, Run, RunRepo, RunStatus, Step, StepRepo, StepStatus};

fn setup() -> (tempfile::TempDir, Db, RunRepo, StepRepo) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let runs = RunRepo::new(&db);
    let steps = StepRepo::new(&db);
    seed_workspace(&db, "ws1");
    (tmp, db, runs, steps)
}

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'github', 100, 100)",
        rusqlite::params![id],
    )
    .unwrap();
}

fn sample_run(id: &str, workspace_id: &str, started_at: i64) -> Run {
    Run {
        id: id.to_string(),
        workspace_id: workspace_id.to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Pending,
        ticket_type: None,
        ticket_ref: None,
        ticket_title: None,
        ticket_body: None,
        backend: "claude-code".to_string(),
        model: "claude-opus-4-7".to_string(),
        started_at,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    }
}

fn sample_step(id: &str, run_id: &str, seq: i64) -> Step {
    Step {
        id: id.to_string(),
        run_id: run_id.to_string(),
        seq,
        agent_name: "architect".to_string(),
        status: StepStatus::Pending,
        started_at: None,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        retry_count: 0,
    }
}

#[test]
fn run_full_happy_path_pending_to_running_to_completed() {
    let (_tmp, _db, runs, _steps) = setup();
    let run = runs.insert(sample_run("run1", "ws1", 100)).expect("insert");
    assert_eq!(run.status, RunStatus::Pending);

    runs.transition("run1", RunStatus::Running)
        .expect("pending -> running");
    let after_running = runs.get("run1").unwrap().unwrap();
    assert_eq!(after_running.status, RunStatus::Running);

    runs.transition("run1", RunStatus::Completed)
        .expect("running -> completed");
    let final_state = runs.get("run1").unwrap().unwrap();
    assert_eq!(final_state.status, RunStatus::Completed);
}

#[test]
fn invalid_transition_pending_to_completed_returns_invalid_state_transition() {
    let (_tmp, _db, runs, _steps) = setup();
    runs.insert(sample_run("run1", "ws1", 100)).unwrap();

    let result = runs.transition("run1", RunStatus::Completed);
    match result {
        Err(CoreError::InvalidStateTransition { from, to }) => {
            assert_eq!(from, "pending");
            assert_eq!(to, "completed");
        }
        Ok(_) => panic!("expected InvalidStateTransition, got Ok"),
        Err(other) => panic!("expected InvalidStateTransition, got {other:?}"),
    }

    // Confirm the status didn't change
    let run = runs.get("run1").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Pending);
}

#[test]
fn list_by_workspace_returns_desc_by_started_at() {
    let (_tmp, _db, runs, _steps) = setup();
    runs.insert(sample_run("r1", "ws1", 100)).unwrap();
    runs.insert(sample_run("r2", "ws1", 300)).unwrap();
    runs.insert(sample_run("r3", "ws1", 200)).unwrap();

    let list = runs.list_by_workspace("ws1", 10).expect("list");
    let ids: Vec<String> = list.into_iter().map(|r| r.id).collect();
    assert_eq!(
        ids,
        vec!["r2".to_string(), "r3".to_string(), "r1".to_string()]
    );
}

#[test]
fn creating_step_for_nonexistent_run_fails_fk() {
    let (_tmp, _db, _runs, steps) = setup();
    // No run inserted first — FK reference to runs(id) should fail.
    let result = steps.insert(sample_step("step1", "nonexistent-run", 0));
    match result {
        Ok(_) => panic!("expected FK violation"),
        Err(CoreError::Db(msg)) => {
            let upper = msg.to_uppercase();
            assert!(
                upper.contains("FOREIGN KEY") || upper.contains("CONSTRAINT"),
                "expected FK/constraint error, got: {msg}"
            );
        }
        Err(other) => panic!("expected Db error, got {other:?}"),
    }
}
