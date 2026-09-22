DROP INDEX IF EXISTS idx_learning_question_resources_question;
-- statement-break
DROP TABLE IF EXISTS learning_question_resources;
-- statement-break
ALTER TABLE learning_question_answers DROP COLUMN max_score;
-- statement-break
ALTER TABLE learning_question_answers DROP COLUMN score;
-- statement-break
ALTER TABLE learning_question_answers DROP COLUMN question_snapshot_json;
-- statement-break
ALTER TABLE learning_question_answers DROP COLUMN answer_payload_json;
-- statement-break
ALTER TABLE learning_questions DROP COLUMN answer_key_json;
-- statement-break
ALTER TABLE learning_questions DROP COLUMN definition_json;
