ALTER TABLE learning_resources ADD COLUMN previous_version_id VARCHAR(36);
-- statement-break
CREATE UNIQUE INDEX uq_learning_resources_previous_version ON learning_resources(previous_version_id);
-- statement-break
ALTER TABLE learning_questions ADD COLUMN previous_version_id VARCHAR(36);
-- statement-break
CREATE UNIQUE INDEX uq_learning_questions_previous_version ON learning_questions(previous_version_id);
