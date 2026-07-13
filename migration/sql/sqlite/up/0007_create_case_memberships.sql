CREATE TABLE case_memberships (
    id TEXT PRIMARY KEY,
    case_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('family', 'commander', 'volunteer')),
    created_by_user_id TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (case_id, user_id),
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE SET NULL
);
-- statement-break
CREATE INDEX idx_case_memberships_user_id ON case_memberships(user_id);
-- statement-break
CREATE INDEX idx_case_memberships_case_role ON case_memberships(case_id, role);
