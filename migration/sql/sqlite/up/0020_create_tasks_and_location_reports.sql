CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    source_clue_id TEXT,
    title TEXT NOT NULL,
    objective TEXT NOT NULL,
    area_text TEXT NOT NULL,
    latitude REAL,
    longitude REAL,
    due_at TEXT NOT NULL,
    background TEXT NOT NULL,
    risk_level TEXT NOT NULL CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
    risk_notes TEXT NOT NULL,
    safety_briefing TEXT NOT NULL,
    expected_feedback TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending_claim', 'assigned', 'accepted', 'active', 'blocked', 'completed', 'cancelled')),
    result_summary TEXT,
    created_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (latitude IS NULL AND longitude IS NULL)
        OR (
            latitude IS NOT NULL
            AND longitude IS NOT NULL
            AND latitude BETWEEN -90 AND 90
            AND longitude BETWEEN -180 AND 180
        )
    ),
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (source_clue_id) REFERENCES clues(id) ON DELETE SET NULL,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_tasks_case_status_due_at ON tasks(case_id, status, due_at);
-- statement-break
CREATE INDEX idx_tasks_source_clue_id ON tasks(source_clue_id);
-- statement-break
CREATE TABLE task_assignments (
    task_id TEXT PRIMARY KEY NOT NULL,
    volunteer_user_id TEXT NOT NULL,
    assigned_by_user_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (task_id, volunteer_user_id),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (assigned_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_task_assignments_volunteer_assigned_at ON task_assignments(volunteer_user_id, assigned_at);
-- statement-break
CREATE TABLE task_location_reports (
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
    FOREIGN KEY (task_id, volunteer_user_id) REFERENCES task_assignments(task_id, volunteer_user_id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_task_location_reports_task_captured_at ON task_location_reports(task_id, captured_at);
-- statement-break
CREATE INDEX idx_task_location_reports_retention_expires_at ON task_location_reports(retention_expires_at);
