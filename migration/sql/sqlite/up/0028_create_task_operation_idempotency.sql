CREATE TABLE task_operation_idempotency (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    volunteer_user_id TEXT NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('location_report', 'task_feedback')),
    idempotency_key TEXT NOT NULL,
    response_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (task_id, volunteer_user_id, operation, idempotency_key),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
