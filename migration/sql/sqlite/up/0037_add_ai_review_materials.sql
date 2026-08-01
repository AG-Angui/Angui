CREATE TABLE intake_profile_drafts (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    parent_draft_id TEXT,
    profile_json TEXT NOT NULL,
    field_metadata_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'confirmed', 'superseded')),
    degradation_status TEXT NOT NULL,
    provider_model TEXT,
    template_version TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    confirmed_by_user_id TEXT,
    confirmed_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES intake_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_draft_id) REFERENCES intake_profile_drafts(id) ON DELETE RESTRICT,
    FOREIGN KEY (confirmed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE UNIQUE INDEX idx_intake_profile_drafts_session_version ON intake_profile_drafts(session_id, version);
-- statement-break
CREATE TABLE case_source_records (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    record_type TEXT NOT NULL CHECK (record_type IN ('message', 'phone_record', 'field_feedback')),
    content TEXT NOT NULL,
    occurred_at TEXT,
    source_reference TEXT,
    created_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_case_source_records_case_created ON case_source_records(case_id, created_at);
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN source_record_id TEXT REFERENCES case_source_records(id) ON DELETE RESTRICT;
-- statement-break
CREATE INDEX idx_clue_drafts_source_record ON clue_drafts(source_record_id);
-- statement-break
CREATE TABLE archive_review_materials (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    parent_material_id TEXT,
    content TEXT NOT NULL,
    source_scope_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'deidentified', 'rejected')),
    created_by_user_id TEXT NOT NULL,
    reviewed_by_user_id TEXT,
    reviewed_at TEXT,
    review_reason TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_material_id) REFERENCES archive_review_materials(id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE UNIQUE INDEX idx_archive_review_materials_case_version ON archive_review_materials(case_id, version);
-- statement-break
ALTER TABLE archive_drafts ADD COLUMN review_material_id TEXT REFERENCES archive_review_materials(id) ON DELETE RESTRICT;
