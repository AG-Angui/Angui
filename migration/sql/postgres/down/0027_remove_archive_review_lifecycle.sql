DROP INDEX IF EXISTS idx_archive_drafts_status;
-- statement-break
ALTER TABLE archive_drafts DROP CONSTRAINT IF EXISTS fk_archive_drafts_deidentifier, DROP CONSTRAINT IF EXISTS fk_archive_drafts_reviewer, DROP CONSTRAINT IF EXISTS archive_drafts_status_check, DROP CONSTRAINT IF EXISTS archive_drafts_deidentification_status_check, DROP CONSTRAINT IF EXISTS archive_drafts_version_check, DROP CONSTRAINT IF EXISTS archive_drafts_usage_scope_check, DROP CONSTRAINT IF EXISTS archive_drafts_retention_status_check, DROP CONSTRAINT IF EXISTS archive_drafts_publication_state_check;
-- statement-break
ALTER TABLE archive_drafts DROP COLUMN IF EXISTS deidentified_by_user_id, DROP COLUMN IF EXISTS deidentified_at, DROP COLUMN IF EXISTS deidentification_reason, DROP COLUMN IF EXISTS reviewed_by_user_id, DROP COLUMN IF EXISTS reviewed_at, DROP COLUMN IF EXISTS review_reason, DROP COLUMN IF EXISTS version, DROP COLUMN IF EXISTS usage_scope, DROP COLUMN IF EXISTS retention_status;
-- statement-break
ALTER TABLE archive_drafts ADD CONSTRAINT archive_drafts_status_check CHECK (status IN ('draft')), ADD CONSTRAINT archive_drafts_deidentification_status_check CHECK (deidentification_status IN ('manual_review_required'));
