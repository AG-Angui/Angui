PRAGMA foreign_keys = OFF;
CREATE TABLE task_assignments_multi (
    task_id TEXT NOT NULL,
    volunteer_user_id TEXT NOT NULL,
    assigned_by_user_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, volunteer_user_id),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (assigned_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
INSERT INTO task_assignments_multi SELECT task_id, volunteer_user_id, assigned_by_user_id, assigned_at, updated_at FROM task_assignments;
DROP TABLE task_assignments;
ALTER TABLE task_assignments_multi RENAME TO task_assignments;
CREATE INDEX idx_task_assignments_volunteer_assigned_at ON task_assignments(volunteer_user_id, assigned_at);
PRAGMA foreign_keys = ON;
