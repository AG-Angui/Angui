CREATE TABLE ai_executions (
    id VARCHAR(36) PRIMARY KEY,
    owner_user_id VARCHAR(36) NOT NULL,
    intake_session_id VARCHAR(36) NULL,
    workflow VARCHAR(80) NOT NULL,
    stage VARCHAR(40) NOT NULL,
    status VARCHAR(30) NOT NULL,
    failure_kind VARCHAR(80) NULL,
    result_status VARCHAR(80) NULL,
    fallback_used BOOLEAN NOT NULL DEFAULT FALSE,
    last_event_id BIGINT NOT NULL DEFAULT 0,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_ai_executions_owner FOREIGN KEY (owner_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT fk_ai_executions_session FOREIGN KEY (intake_session_id) REFERENCES intake_sessions(id) ON UPDATE CASCADE ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_ai_executions_owner ON ai_executions(owner_user_id, updated_at);
-- statement-break
CREATE TABLE ai_execution_events (
    id VARCHAR(36) PRIMARY KEY,
    execution_id VARCHAR(36) NOT NULL,
    event_id BIGINT NOT NULL,
    event_type VARCHAR(80) NOT NULL,
    stage VARCHAR(40) NULL,
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_ai_execution_events_execution FOREIGN KEY (execution_id) REFERENCES ai_executions(id) ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE KEY uq_ai_execution_events_sequence (execution_id, event_id)
);
-- statement-break
CREATE INDEX idx_ai_execution_events_execution ON ai_execution_events(execution_id, event_id);
