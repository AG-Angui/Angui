DROP TABLE IF EXISTS intake_profile_drafts;
-- statement-break
DROP TABLE IF EXISTS archive_review_materials;
-- statement-break
DROP INDEX IF EXISTS idx_case_source_records_case_created;
-- statement-break
DROP TABLE IF EXISTS case_source_records;
-- statement-break
DROP INDEX IF EXISTS idx_clue_drafts_source_record;
-- statement-break
ALTER TABLE clue_drafts DROP COLUMN source_record_id;
-- statement-break
ALTER TABLE archive_drafts DROP COLUMN review_material_id;
