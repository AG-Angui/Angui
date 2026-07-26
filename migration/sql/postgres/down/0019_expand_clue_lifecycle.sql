DROP TABLE IF EXISTS clue_attachment_links;
-- statement-break
DROP INDEX IF EXISTS idx_clues_related_clue_id;
-- statement-break
ALTER TABLE clues DROP COLUMN review_reason, DROP COLUMN relationship_type, DROP COLUMN related_clue_id, DROP COLUMN linked_task_reference, DROP COLUMN next_action, DROP COLUMN location_precision, DROP COLUMN confirmed_at, DROP COLUMN reported_at, DROP COLUMN raw_record_reference, DROP COLUMN source_type;
