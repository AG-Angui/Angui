CREATE TABLE case_memberships (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36) NOT NULL,
    role VARCHAR(32) NOT NULL CHECK (role IN ('family', 'commander', 'volunteer')),
    created_by_user_id VARCHAR(36),
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT uq_case_memberships_case_user UNIQUE (case_id, user_id),
    CONSTRAINT fk_case_memberships_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    CONSTRAINT fk_case_memberships_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_case_memberships_creator FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE SET NULL
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_case_memberships_user_id ON case_memberships(user_id);
-- statement-break
CREATE INDEX idx_case_memberships_case_role ON case_memberships(case_id, role);
