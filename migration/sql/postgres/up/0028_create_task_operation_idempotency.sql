CREATE TABLE task_operation_idempotency (
    id VARCHAR(36) PRIMARY KEY,
    task_id VARCHAR(36) NOT NULL,
    volunteer_user_id VARCHAR(36) NOT NULL,
    operation VARCHAR(32) NOT NULL CHECK (operation IN ('location_report', 'task_feedback')),
    idempotency_key VARCHAR(36) NOT NULL,
    response_json TEXT NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT uq_task_operation_idempotency UNIQUE (task_id, volunteer_user_id, operation, idempotency_key),
    CONSTRAINT fk_task_operation_idempotency_task FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_operation_idempotency_volunteer FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
