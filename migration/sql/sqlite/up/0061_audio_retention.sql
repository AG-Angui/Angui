ALTER TABLE voice_reports ADD COLUMN audio_deleted_at TEXT NULL;
-- statement-break
ALTER TABLE intercom_recordings ADD COLUMN audio_deleted_at TEXT NULL;
