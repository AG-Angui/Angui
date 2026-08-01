CREATE TABLE intake_profile_drafts (id VARCHAR(36) PRIMARY KEY, session_id VARCHAR(36) NOT NULL, version INT NOT NULL, parent_draft_id VARCHAR(36) NULL, profile_json TEXT NOT NULL, field_metadata_json TEXT NOT NULL, status VARCHAR(32) NOT NULL, degradation_status VARCHAR(64) NOT NULL, provider_model TEXT NULL, template_version VARCHAR(128) NOT NULL, generated_at VARCHAR(40) NOT NULL, confirmed_by_user_id VARCHAR(36) NULL, confirmed_at VARCHAR(40) NULL, created_at VARCHAR(40) NOT NULL, CONSTRAINT chk_intake_profile_drafts_version CHECK (version >= 1), CONSTRAINT chk_intake_profile_drafts_status CHECK (status IN ('draft', 'confirmed', 'superseded')), FOREIGN KEY (session_id) REFERENCES intake_sessions(id) ON DELETE CASCADE, FOREIGN KEY (parent_draft_id) REFERENCES intake_profile_drafts(id) ON DELETE RESTRICT, FOREIGN KEY (confirmed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT);
-- statement-break
CREATE UNIQUE INDEX idx_intake_profile_drafts_session_version ON intake_profile_drafts(session_id, version);
-- statement-break
CREATE TABLE case_source_records (id VARCHAR(36) PRIMARY KEY, case_id VARCHAR(36) NOT NULL, record_type VARCHAR(32) NOT NULL, content TEXT NOT NULL, occurred_at VARCHAR(40) NULL, source_reference TEXT NULL, created_by_user_id VARCHAR(36) NOT NULL, created_at VARCHAR(40) NOT NULL, CONSTRAINT chk_case_source_records_type CHECK (record_type IN ('message', 'phone_record', 'field_feedback')), FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE, FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT);
-- statement-break
CREATE INDEX idx_case_source_records_case_created ON case_source_records(case_id, created_at);
-- statement-break
ALTER TABLE clue_drafts ADD COLUMN source_record_id VARCHAR(36) NULL, ADD CONSTRAINT fk_clue_drafts_source_record FOREIGN KEY (source_record_id) REFERENCES case_source_records(id) ON DELETE RESTRICT;
-- statement-break
CREATE INDEX idx_clue_drafts_source_record ON clue_drafts(source_record_id);
-- statement-break
CREATE TABLE archive_review_materials (id VARCHAR(36) PRIMARY KEY, case_id VARCHAR(36) NOT NULL, version INT NOT NULL, parent_material_id VARCHAR(36) NULL, content TEXT NOT NULL, source_scope_json TEXT NOT NULL, status VARCHAR(32) NOT NULL, created_by_user_id VARCHAR(36) NOT NULL, reviewed_by_user_id VARCHAR(36) NULL, reviewed_at VARCHAR(40) NULL, review_reason TEXT NULL, created_at VARCHAR(40) NOT NULL, CONSTRAINT chk_archive_review_materials_version CHECK (version >= 1), CONSTRAINT chk_archive_review_materials_status CHECK (status IN ('draft', 'deidentified', 'rejected')), FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE, FOREIGN KEY (parent_material_id) REFERENCES archive_review_materials(id) ON DELETE RESTRICT, FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT, FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT);
-- statement-break
CREATE UNIQUE INDEX idx_archive_review_materials_case_version ON archive_review_materials(case_id, version);
-- statement-break
ALTER TABLE archive_drafts ADD COLUMN review_material_id VARCHAR(36) NULL, ADD CONSTRAINT fk_archive_drafts_review_material FOREIGN KEY (review_material_id) REFERENCES archive_review_materials(id) ON DELETE RESTRICT;
