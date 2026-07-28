UPDATE summary_drafts AS draft
JOIN summary_drafts AS newer
  ON newer.case_id = draft.case_id
 AND newer.status = 'published'
 AND (
     newer.updated_at > draft.updated_at
     OR (newer.updated_at = draft.updated_at AND newer.id > draft.id)
 )
SET draft.status = 'superseded'
WHERE draft.status = 'published';
-- statement-break
ALTER TABLE summary_drafts
    ADD COLUMN published_case_id VARCHAR(36)
        GENERATED ALWAYS AS (CASE WHEN status = 'published' THEN case_id ELSE NULL END) STORED;
-- statement-break
CREATE UNIQUE INDEX idx_summary_drafts_one_published_per_case
    ON summary_drafts(published_case_id);
