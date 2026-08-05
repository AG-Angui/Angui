CREATE TABLE ai_executions (
    id TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL,
    intake_session_id TEXT,
    workflow TEXT NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL,
    failure_kind TEXT,
    result_status TEXT,
    fallback_used INTEGER NOT NULL DEFAULT 0,
    last_event_id INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY (intake_session_id) REFERENCES intake_sessions(id) ON UPDATE CASCADE ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_ai_executions_owner ON ai_executions(owner_user_id, updated_at DESC);
-- statement-break
CREATE TABLE ai_execution_events (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    stage TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES ai_executions(id) ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE (execution_id, event_id)
);
