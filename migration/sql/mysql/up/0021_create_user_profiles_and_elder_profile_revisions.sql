CREATE TABLE user_profiles (
    user_id VARCHAR(36) PRIMARY KEY,
    avatar_reference VARCHAR(500),
    preferences_json TEXT NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_user_profiles_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB;
-- statement-break
CREATE TABLE elder_profile_revisions (
    id VARCHAR(36) PRIMARY KEY,
    elder_profile_id VARCHAR(36) NOT NULL,
    case_id VARCHAR(36) NOT NULL,
    updated_by_user_id VARCHAR(36) NOT NULL,
    previous_profile_json TEXT NOT NULL,
    updated_profile_json TEXT NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_elder_profile_revisions_profile FOREIGN KEY (elder_profile_id) REFERENCES elder_profiles(id) ON DELETE CASCADE,
    CONSTRAINT fk_elder_profile_revisions_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    CONSTRAINT fk_elder_profile_revisions_actor FOREIGN KEY (updated_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_elder_profile_revisions_case_created_at ON elder_profile_revisions(case_id, created_at);
