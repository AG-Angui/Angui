ALTER TABLE clues ADD COLUMN source_type TEXT NOT NULL DEFAULT 'manual_report';
-- statement-break
ALTER TABLE clues ADD COLUMN raw_record_reference TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN reported_at TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN confirmed_at TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN location_precision TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN next_action TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN linked_task_reference TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN related_clue_id TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN relationship_type TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN review_reason TEXT;
-- statement-break
UPDATE clues SET reported_at = created_at WHERE reported_at IS NULL;
-- statement-break
CREATE INDEX idx_clues_related_clue_id ON clues(related_clue_id);
-- statement-break
CREATE TABLE clue_attachment_links (
    clue_id TEXT NOT NULL,
    attachment_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (clue_id, attachment_id),
    FOREIGN KEY (clue_id) REFERENCES clues(id) ON DELETE CASCADE,
    FOREIGN KEY (attachment_id) REFERENCES case_attachments(id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_clue_attachment_links_attachment_id ON clue_attachment_links(attachment_id);
