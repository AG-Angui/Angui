CREATE TABLE learning_content_review_events (
    id TEXT PRIMARY KEY NOT NULL,
    content_type TEXT NOT NULL CHECK (content_type IN ('resource', 'question')),
    content_id TEXT NOT NULL,
    content_version INTEGER NOT NULL CHECK (content_version >= 1),
    event_type TEXT NOT NULL CHECK (event_type IN ('submitted', 'deidentified', 'reviewed', 'published', 'withdrawn', 'rejected')),
    actor_user_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    permitted_use TEXT NOT NULL CHECK (permitted_use IN ('training', 'public_information')),
    created_at TEXT NOT NULL,
    FOREIGN KEY (actor_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_learning_content_review_events_content ON learning_content_review_events(content_type, content_id, content_version, created_at);
