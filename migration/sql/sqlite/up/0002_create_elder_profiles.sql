CREATE TABLE elder_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    age INTEGER,
    gender TEXT,
    physical_description TEXT,
    clothing_description TEXT,
    health_notes TEXT,
    last_seen_at TEXT,
    last_seen_location TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE
);
