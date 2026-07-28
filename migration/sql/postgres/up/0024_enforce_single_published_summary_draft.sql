UPDATE summary_drafts AS draft
SET status = 'superseded'
FROM summary_drafts AS newer
WHERE draft.status = 'published'
  AND newer.case_id = draft.case_id
  AND newer.status = 'published'
  AND (
      newer.updated_at > draft.updated_at
      OR (newer.updated_at = draft.updated_at AND newer.id > draft.id)
  );
-- statement-break
CREATE UNIQUE INDEX idx_summary_drafts_one_published_per_case
    ON summary_drafts(case_id)
    WHERE status = 'published';
