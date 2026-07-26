CREATE TABLE tasks (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL,
    source_clue_id VARCHAR(36),
    title VARCHAR(200) NOT NULL,
    objective TEXT NOT NULL,
    area_text VARCHAR(500) NOT NULL,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    due_at VARCHAR(40) NOT NULL,
    background TEXT NOT NULL,
    risk_level VARCHAR(16) NOT NULL CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
    risk_notes TEXT NOT NULL,
    safety_briefing TEXT NOT NULL,
    expected_feedback TEXT NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('pending_claim', 'assigned', 'accepted', 'active', 'blocked', 'completed', 'cancelled')),
    result_summary TEXT,
    created_by_user_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CHECK (
        (latitude IS NULL AND longitude IS NULL)
        OR (
            latitude IS NOT NULL
            AND longitude IS NOT NULL
            AND latitude BETWEEN -90 AND 90
            AND longitude BETWEEN -180 AND 180
        )
    ),
    CONSTRAINT fk_tasks_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    CONSTRAINT fk_tasks_source_clue FOREIGN KEY (source_clue_id) REFERENCES clues(id) ON DELETE SET NULL,
    CONSTRAINT fk_tasks_creator FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_tasks_case_status_due_at ON tasks(case_id, status, due_at);
-- statement-break
CREATE INDEX idx_tasks_source_clue_id ON tasks(source_clue_id);
-- statement-break
CREATE TABLE task_assignments (
    task_id VARCHAR(36) PRIMARY KEY,
    volunteer_user_id VARCHAR(36) NOT NULL,
    assigned_by_user_id VARCHAR(36) NOT NULL,
    assigned_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    UNIQUE (task_id, volunteer_user_id),
    CONSTRAINT fk_task_assignments_task FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_assignments_volunteer FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    CONSTRAINT fk_task_assignments_assigner FOREIGN KEY (assigned_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_task_assignments_volunteer_assigned_at ON task_assignments(volunteer_user_id, assigned_at);
-- statement-break
CREATE TABLE task_location_reports (
    id VARCHAR(36) PRIMARY KEY,
    task_id VARCHAR(36) NOT NULL,
    volunteer_user_id VARCHAR(36) NOT NULL,
    source VARCHAR(16) NOT NULL CHECK (source = 'simulated'),
    latitude DOUBLE PRECISION NOT NULL CHECK (latitude BETWEEN -90 AND 90),
    longitude DOUBLE PRECISION NOT NULL CHECK (longitude BETWEEN -180 AND 180),
    accuracy_meters DOUBLE PRECISION NOT NULL CHECK (accuracy_meters >= 0 AND accuracy_meters <= 10000),
    captured_at VARCHAR(40) NOT NULL,
    retention_expires_at VARCHAR(40) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_task_location_reports_task FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_location_reports_assignment FOREIGN KEY (task_id, volunteer_user_id) REFERENCES task_assignments(task_id, volunteer_user_id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_task_location_reports_task_captured_at ON task_location_reports(task_id, captured_at);
-- statement-break
CREATE INDEX idx_task_location_reports_retention_expires_at ON task_location_reports(retention_expires_at);
