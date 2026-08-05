CREATE TABLE ai_executions (
    id VARCHAR(36) PRIMARY KEY,
    owner_user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    intake_session_id VARCHAR(36) REFERENCES intake_sessions(id) ON UPDATE CASCADE ON DELETE SET NULL,
    workflow VARCHAR(80) NOT NULL,
    stage VARCHAR(40) NOT NULL,
    status VARCHAR(30) NOT NULL,
    failure_kind VARCHAR(80),
    result_status VARCHAR(80),
    fallback_used BOOLEAN NOT NULL DEFAULT FALSE,
    last_event_id BIGINT NOT NULL DEFAULT 0,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL
);
-- statement-break
CREATE INDEX idx_ai_executions_owner ON ai_executions(owner_user_id, updated_at DESC);
-- statement-break
CREATE TABLE ai_execution_events (
    id VARCHAR(36) PRIMARY KEY,
    execution_id VARCHAR(36) NOT NULL REFERENCES ai_executions(id) ON UPDATE CASCADE ON DELETE CASCADE,
    event_id BIGINT NOT NULL,
    event_type VARCHAR(80) NOT NULL,
    stage VARCHAR(40),
    created_at VARCHAR(40) NOT NULL,
    UNIQUE (execution_id, event_id)
);
