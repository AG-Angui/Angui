DROP INDEX uq_learning_questions_previous_version;
-- statement-break
DROP INDEX uq_learning_resources_previous_version;
-- statement-break
ALTER TABLE learning_questions DROP COLUMN previous_version_id;
-- statement-break
ALTER TABLE learning_resources DROP COLUMN previous_version_id;
