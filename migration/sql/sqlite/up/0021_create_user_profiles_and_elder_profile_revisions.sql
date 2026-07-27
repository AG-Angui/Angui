CREATE TABLE user_profiles (
    user_id TEXT PRIMARY KEY NOT NULL,
    avatar_reference TEXT,
    preferences_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
-- statement-break
CREATE TABLE elder_profile_revisions (
    id TEXT PRIMARY KEY NOT NULL,
    elder_profile_id TEXT NOT NULL,
    case_id TEXT NOT NULL,
    updated_by_user_id TEXT NOT NULL,
    previous_profile_json TEXT NOT NULL,
    updated_profile_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (elder_profile_id) REFERENCES elder_profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (updated_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_elder_profile_revisions_case_created_at ON elder_profile_revisions(case_id, created_at);
