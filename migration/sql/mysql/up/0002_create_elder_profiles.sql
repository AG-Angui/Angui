CREATE TABLE elder_profiles (
    id VARCHAR(36) PRIMARY KEY,
    case_id VARCHAR(36) NOT NULL UNIQUE,
    display_name VARCHAR(120) NOT NULL,
    age SMALLINT,
    gender VARCHAR(32),
    physical_description TEXT,
    clothing_description TEXT,
    health_notes TEXT,
    last_seen_at VARCHAR(40),
    last_seen_location VARCHAR(500),
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_elder_profiles_case FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE
) ENGINE=InnoDB;
