-- Event log (full replay fidelity)
CREATE TABLE stream_events (
    run_id       TEXT NOT NULL,
    step_id      TEXT,
    seq          INTEGER NOT NULL,
    event_type   TEXT NOT NULL,
    payload      BLOB NOT NULL,            -- MessagePack-encoded Event
    timestamp_ms INTEGER NOT NULL,
    PRIMARY KEY (run_id, seq)
);

CREATE INDEX idx_stream_events_step ON stream_events(step_id, seq);
