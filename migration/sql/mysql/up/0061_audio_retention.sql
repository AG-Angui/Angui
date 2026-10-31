ALTER TABLE voice_reports ADD COLUMN audio_deleted_at VARCHAR(40) NULL;
-- statement-break
ALTER TABLE intercom_recordings ADD COLUMN audio_deleted_at VARCHAR(40) NULL;
