CREATE TABLE task_applications (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    volunteer_user_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'withdrawn')),
    note TEXT NULL,
    reviewed_by_user_id TEXT NULL,
    reviewed_at TEXT NULL,
    review_reason TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (task_id, volunteer_user_id),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
CREATE INDEX idx_task_applications_task_status ON task_applications(task_id, status, created_at);
