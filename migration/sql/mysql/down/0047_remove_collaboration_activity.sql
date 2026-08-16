DROP TABLE IF EXISTS voice_transcripts;
-- statement-break
DROP TABLE IF EXISTS voice_reports;
-- statement-break
DROP INDEX idx_space_messages_space_sent ON space_messages;
-- statement-break
DROP TABLE IF EXISTS space_messages;
-- statement-break
DROP TABLE IF EXISTS space_arrivals;
-- statement-break
DROP INDEX idx_space_location_samples_window ON space_location_samples;
-- statement-break
DROP TABLE IF EXISTS space_location_samples;
