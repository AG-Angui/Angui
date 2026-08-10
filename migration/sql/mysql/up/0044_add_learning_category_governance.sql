CREATE TABLE learning_categories (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(160) NOT NULL,
    normalized_name VARCHAR(160) NOT NULL UNIQUE,
    status VARCHAR(16) NOT NULL,
    submitted_by_user_id VARCHAR(64) NOT NULL,
    reviewed_by_user_id VARCHAR(64) NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT chk_learning_categories_status CHECK (status IN ('pending', 'enabled', 'rejected', 'disabled')),
    CONSTRAINT fk_learning_categories_submitter FOREIGN KEY (submitted_by_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT fk_learning_categories_reviewer FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_learning_categories_status_name ON learning_categories(status, name);
-- statement-break
CREATE TABLE learning_category_review_events (
    id VARCHAR(64) PRIMARY KEY,
    category_id VARCHAR(64) NOT NULL,
    event_type VARCHAR(16) NOT NULL,
    actor_user_id VARCHAR(64) NOT NULL,
    reason TEXT NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    CONSTRAINT chk_learning_category_review_events_type CHECK (event_type IN ('submitted', 'enabled', 'rejected', 'disabled')),
    CONSTRAINT fk_learning_category_review_events_category FOREIGN KEY (category_id) REFERENCES learning_categories(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT fk_learning_category_review_events_actor FOREIGN KEY (actor_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_learning_category_review_events_category_created ON learning_category_review_events(category_id, created_at);
-- statement-break
ALTER TABLE learning_resources ADD COLUMN category_id VARCHAR(64) NULL;
-- statement-break
ALTER TABLE learning_resources ADD COLUMN category_name VARCHAR(160) NULL;
-- statement-break
ALTER TABLE learning_resources ADD CONSTRAINT fk_learning_resources_category FOREIGN KEY (category_id) REFERENCES learning_categories(id) ON UPDATE CASCADE ON DELETE RESTRICT;
-- statement-break
CREATE INDEX idx_learning_resources_category ON learning_resources(category_id);
