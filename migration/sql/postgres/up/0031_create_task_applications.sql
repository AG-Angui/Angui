CREATE TABLE task_applications (
    id VARCHAR(36) PRIMARY KEY,
    task_id VARCHAR(36) NOT NULL,
    volunteer_user_id VARCHAR(36) NOT NULL,
    status VARCHAR(16) NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'withdrawn')),
    note TEXT NULL,
    reviewed_by_user_id VARCHAR(36) NULL,
    reviewed_at VARCHAR(40) NULL,
    review_reason TEXT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT uq_task_applications_volunteer UNIQUE (task_id, volunteer_user_id),
    CONSTRAINT fk_task_applications_task FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_applications_volunteer FOREIGN KEY (volunteer_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    CONSTRAINT fk_task_applications_reviewer FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
CREATE INDEX idx_task_applications_task_status ON task_applications(task_id, status, created_at);
