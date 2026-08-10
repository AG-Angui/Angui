CREATE TABLE learning_categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'enabled', 'rejected', 'disabled')),
    submitted_by_user_id TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    reviewed_by_user_id TEXT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
CREATE INDEX idx_learning_categories_status_name ON learning_categories(status, name);
-- statement-break
CREATE TABLE learning_category_review_events (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL REFERENCES learning_categories(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    event_type TEXT NOT NULL CHECK (event_type IN ('submitted', 'enabled', 'rejected', 'disabled')),
    actor_user_id TEXT NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL
);
-- statement-break
CREATE INDEX idx_learning_category_review_events_category_created ON learning_category_review_events(category_id, created_at);
-- statement-break
ALTER TABLE learning_resources ADD COLUMN category_id TEXT NULL REFERENCES learning_categories(id) ON UPDATE CASCADE ON DELETE RESTRICT;
-- statement-break
ALTER TABLE learning_resources ADD COLUMN category_name TEXT NULL;
-- statement-break
CREATE INDEX idx_learning_resources_category ON learning_resources(category_id);
