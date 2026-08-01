CREATE TABLE intake_profile_drafts (id VARCHAR(36) PRIMARY KEY, session_id VARCHAR(36) NOT NULL REFERENCES intake_sessions(id) ON DELETE CASCADE, version INTEGER NOT NULL CHECK (version >= 1), parent_draft_id VARCHAR(36) REFERENCES intake_profile_drafts(id) ON DELETE RESTRICT, profile_json TEXT NOT NULL, field_metadata_json TEXT NOT NULL, status VARCHAR(32) NOT NULL CHECK (status IN ('draft', 'confirmed', 'superseded')), degradation_status VARCHAR(64) NOT NULL, provider_model TEXT, template_version VARCHAR(128) NOT NULL, generated_at VARCHAR(40) NOT NULL, confirmed_by_user_id VARCHAR(36) REFERENCES users(id) ON DELETE RESTRICT, confirmed_at VARCHAR(40), created_at VARCHAR(40) NOT NULL);
-- statement-break
CREATE UNIQUE INDEX idx_intake_profile_drafts_session_version ON intake_profile_drafts(session_id, version);
-- statement-break
CREATE TABLE case_source_records (id VARCHAR(36) PRIMARY KEY, case_id VARCHAR(36) NOT NULL REFERENCES cases(id) ON DELETE CASCADE, record_type VARCHAR(32) NOT NULL CHECK (record_type IN ('message', 'phone_record', 'field_feedback')), content TEXT NOT NULL, occurred_at VARCHAR(40), source_reference TEXT, created_by_user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE RESTRICT, created_at VARCHAR(40) NOT NULL);
-- statement-break
CREATE INDEX idx_case_source_records_case_created ON case_source_records(case_id, created_at);
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN source_record_id VARCHAR(36) REFERENCES case_source_records(id) ON DELETE RESTRICT;
-- statement-break
CREATE INDEX idx_clue_drafts_source_record ON clue_drafts(source_record_id);
-- statement-break
CREATE TABLE archive_review_materials (id VARCHAR(36) PRIMARY KEY, case_id VARCHAR(36) NOT NULL REFERENCES cases(id) ON DELETE CASCADE, version INTEGER NOT NULL CHECK (version >= 1), parent_material_id VARCHAR(36) REFERENCES archive_review_materials(id) ON DELETE RESTRICT, content TEXT NOT NULL, source_scope_json TEXT NOT NULL, status VARCHAR(32) NOT NULL CHECK (status IN ('draft', 'deidentified', 'rejected')), created_by_user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE RESTRICT, reviewed_by_user_id VARCHAR(36) REFERENCES users(id) ON DELETE RESTRICT, reviewed_at VARCHAR(40), review_reason TEXT, created_at VARCHAR(40) NOT NULL);
-- statement-break
CREATE UNIQUE INDEX idx_archive_review_materials_case_version ON archive_review_materials(case_id, version);
-- statement-break
ALTER TABLE archive_drafts ADD COLUMN review_material_id VARCHAR(36) REFERENCES archive_review_materials(id) ON DELETE RESTRICT;
