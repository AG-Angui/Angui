DROP TABLE IF EXISTS learning_question_resources;
-- statement-break
ALTER TABLE learning_question_answers DROP COLUMN max_score, DROP COLUMN score, DROP COLUMN question_snapshot_json, DROP COLUMN answer_payload_json;
-- statement-break
ALTER TABLE learning_questions DROP COLUMN answer_key_json, DROP COLUMN definition_json;
