ALTER TABLE archive_drafts DROP FOREIGN KEY fk_archive_drafts_review_material, DROP COLUMN review_material_id;
-- statement-break
DROP TABLE archive_review_materials;
-- statement-break
ALTER TABLE clue_drafts DROP FOREIGN KEY fk_clue_drafts_source_record, DROP COLUMN source_record_id;
-- statement-break
DROP TABLE case_source_records;
-- statement-break
DROP TABLE intake_profile_drafts;
