DROP TABLE IF EXISTS clue_attachment_links;
-- statement-break
DROP INDEX IF EXISTS idx_clues_related_clue_id;
-- statement-break
ALTER TABLE clues DROP COLUMN review_reason;
-- statement-break
ALTER TABLE clues DROP COLUMN relationship_type;
-- statement-break
ALTER TABLE clues DROP COLUMN related_clue_id;
-- statement-break
ALTER TABLE clues DROP COLUMN linked_task_reference;
-- statement-break
ALTER TABLE clues DROP COLUMN next_action;
-- statement-break
ALTER TABLE clues DROP COLUMN location_precision;
-- statement-break
ALTER TABLE clues DROP COLUMN confirmed_at;
-- statement-break
ALTER TABLE clues DROP COLUMN reported_at;
-- statement-break
ALTER TABLE clues DROP COLUMN raw_record_reference;
-- statement-break
ALTER TABLE clues DROP COLUMN source_type;
