CREATE TABLE space_location_samples (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, latitude REAL NOT NULL CHECK (latitude BETWEEN -90 AND 90), longitude REAL NOT NULL CHECK (longitude BETWEEN -180 AND 180), accuracy_meters REAL NOT NULL CHECK (accuracy_meters >= 0), captured_at TEXT NOT NULL, operation_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL);
-- statement-break
CREATE INDEX idx_space_location_samples_window ON space_location_samples(space_id, user_id, captured_at);
-- statement-break
CREATE TABLE space_arrivals (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, target_type TEXT NOT NULL CHECK (target_type IN ('task','rally_point','area')), target_id TEXT NOT NULL, arrived_at TEXT NOT NULL, accuracy_meters REAL NOT NULL CHECK (accuracy_meters >= 0), UNIQUE(space_id, user_id, target_type, target_id));
-- statement-break
CREATE TABLE space_messages (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, sender_id TEXT NOT NULL REFERENCES users(id), message_type TEXT NOT NULL CHECK (message_type IN ('text','broadcast')), content TEXT NOT NULL, sent_at TEXT NOT NULL, recalled_at TEXT NULL, UNIQUE(space_id, id));
-- statement-break
CREATE INDEX idx_space_messages_space_sent ON space_messages(space_id, sent_at);
-- statement-break
CREATE TABLE voice_reports (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, case_id TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE, reporter_id TEXT NOT NULL REFERENCES users(id), object_key TEXT NOT NULL UNIQUE, content_type TEXT NOT NULL, byte_size INTEGER NOT NULL CHECK (byte_size > 0), status TEXT NOT NULL CHECK (status IN ('uploaded','transcribing','transcribed','draft_ready','failed','reviewed')), created_at TEXT NOT NULL, failed_reason TEXT NULL);
-- statement-break
CREATE TABLE voice_transcripts (id TEXT PRIMARY KEY, voice_report_id TEXT NOT NULL UNIQUE REFERENCES voice_reports(id) ON DELETE CASCADE, content TEXT NOT NULL, provider TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('completed','failed')), created_at TEXT NOT NULL);
