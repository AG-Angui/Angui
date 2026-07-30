PRAGMA foreign_keys = OFF;
CREATE TABLE task_assignments_single (
    task_id TEXT PRIMARY KEY NOT NULL,
    volunteer_user_id TEXT NOT NULL,
    assigned_by_user_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (assigned_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
INSERT INTO task_assignments_single SELECT task_id, MIN(volunteer_user_id), MIN(assigned_by_user_id), MIN(assigned_at), MAX(updated_at) FROM task_assignments GROUP BY task_id;
DROP TABLE task_assignments;
ALTER TABLE task_assignments_single RENAME TO task_assignments;
CREATE INDEX idx_task_assignments_volunteer_assigned_at ON task_assignments(volunteer_user_id, assigned_at);
PRAGMA foreign_keys = ON;
