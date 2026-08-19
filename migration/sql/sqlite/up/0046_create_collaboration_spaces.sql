CREATE TABLE collaboration_spaces (id TEXT PRIMARY KEY, case_id TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE, name TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('active','archived')), created_by_user_id TEXT NOT NULL REFERENCES users(id), created_at TEXT NOT NULL, archived_at TEXT NULL, next_event_version INTEGER NOT NULL DEFAULT 0 CHECK (next_event_version >= 0));
-- statement-break
CREATE INDEX idx_collaboration_spaces_case_status ON collaboration_spaces(case_id, status, created_at);
-- statement-break
CREATE TABLE space_members (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id), role TEXT NOT NULL CHECK (role IN ('commander','volunteer')), status TEXT NOT NULL CHECK (status IN ('active','left')), joined_at TEXT NOT NULL, left_at TEXT NULL, UNIQUE(space_id, user_id));
-- statement-break
CREATE INDEX idx_space_members_user_status ON space_members(user_id, status);
-- statement-break
CREATE TABLE space_member_slots (user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, slot INTEGER NOT NULL CHECK (slot BETWEEN 1 AND 3), member_id TEXT NOT NULL UNIQUE REFERENCES space_members(id) ON DELETE CASCADE, PRIMARY KEY(user_id, slot));
-- statement-break
CREATE TABLE space_location_consents (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, user_id TEXT NOT NULL REFERENCES users(id), member_id TEXT NOT NULL UNIQUE REFERENCES space_members(id) ON DELETE CASCADE, consent_version TEXT NOT NULL, granted_at TEXT NOT NULL, revoked_at TEXT NULL, UNIQUE(space_id, user_id));
-- statement-break
CREATE TABLE space_events (id TEXT PRIMARY KEY, space_id TEXT NOT NULL REFERENCES collaboration_spaces(id) ON DELETE CASCADE, case_id TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE, event_type TEXT NOT NULL, version INTEGER NOT NULL CHECK (version > 0), visibility_scope TEXT NOT NULL CHECK (visibility_scope IN ('space_members','commanders','self')), payload_json TEXT NOT NULL, occurred_at TEXT NOT NULL, UNIQUE(space_id, version));
-- statement-break
CREATE INDEX idx_space_events_space_version ON space_events(space_id, version);
-- statement-break
CREATE TABLE event_outbox (id TEXT PRIMARY KEY, event_id TEXT NOT NULL UNIQUE REFERENCES space_events(id) ON DELETE CASCADE, topic TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('pending','delivered','failed')), attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0), available_at TEXT NOT NULL, delivered_at TEXT NULL, created_at TEXT NOT NULL);
-- statement-break
CREATE INDEX idx_event_outbox_pending ON event_outbox(status, available_at);
