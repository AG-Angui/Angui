ALTER TABLE clues ADD COLUMN source_type VARCHAR(32) NOT NULL DEFAULT 'manual_report';
-- statement-break
ALTER TABLE clues ADD COLUMN raw_record_reference VARCHAR(500);
-- statement-break
ALTER TABLE clues ADD COLUMN reported_at VARCHAR(40);
-- statement-break
ALTER TABLE clues ADD COLUMN confirmed_at VARCHAR(40);
-- statement-break
ALTER TABLE clues ADD COLUMN location_precision VARCHAR(32);
-- statement-break
ALTER TABLE clues ADD COLUMN next_action VARCHAR(500);
-- statement-break
ALTER TABLE clues ADD COLUMN linked_task_reference VARCHAR(120);
-- statement-break
ALTER TABLE clues ADD COLUMN related_clue_id VARCHAR(36);
-- statement-break
ALTER TABLE clues ADD COLUMN relationship_type VARCHAR(32);
-- statement-break
ALTER TABLE clues ADD COLUMN review_reason VARCHAR(1000);
-- statement-break
UPDATE clues SET reported_at = created_at WHERE reported_at IS NULL;
-- statement-break
ALTER TABLE clues ALTER COLUMN reported_at SET NOT NULL;
-- statement-break
CREATE INDEX idx_clues_related_clue_id ON clues(related_clue_id);
-- statement-break
CREATE TABLE clue_attachment_links (
    clue_id VARCHAR(36) NOT NULL,
    attachment_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    PRIMARY KEY (clue_id, attachment_id),
    CONSTRAINT fk_clue_attachment_links_clue FOREIGN KEY (clue_id) REFERENCES clues(id) ON DELETE CASCADE,
    CONSTRAINT fk_clue_attachment_links_attachment FOREIGN KEY (attachment_id) REFERENCES case_attachments(id) ON DELETE CASCADE
);
-- statement-break
CREATE INDEX idx_clue_attachment_links_attachment_id ON clue_attachment_links(attachment_id);
