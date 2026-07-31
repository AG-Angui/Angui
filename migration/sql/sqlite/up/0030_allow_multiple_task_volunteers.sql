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
CREATE TABLE task_location_reports_multi (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    volunteer_user_id TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source = 'simulated'),
    latitude REAL NOT NULL CHECK (latitude BETWEEN -90 AND 90),
    longitude REAL NOT NULL CHECK (longitude BETWEEN -180 AND 180),
    accuracy_meters REAL NOT NULL CHECK (accuracy_meters >= 0 AND accuracy_meters <= 10000),
    captured_at TEXT NOT NULL,
    retention_expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id, volunteer_user_id) REFERENCES task_assignments_multi(task_id, volunteer_user_id) ON DELETE CASCADE
);
INSERT INTO task_location_reports_multi SELECT id, task_id, volunteer_user_id, source, latitude, longitude, accuracy_meters, captured_at, retention_expires_at, created_at FROM task_location_reports;
DROP TABLE task_location_reports;
DROP TABLE task_assignments;
ALTER TABLE task_assignments_multi RENAME TO task_assignments;
ALTER TABLE task_location_reports_multi RENAME TO task_location_reports;
CREATE INDEX idx_task_assignments_volunteer_assigned_at ON task_assignments(volunteer_user_id, assigned_at);
CREATE INDEX idx_task_location_reports_task_captured_at ON task_location_reports(task_id, captured_at);
CREATE INDEX idx_task_location_reports_retention_expires_at ON task_location_reports(retention_expires_at);
PRAGMA foreign_keys = ON;
