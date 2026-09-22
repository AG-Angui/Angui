ALTER TABLE learning_questions ADD COLUMN definition_json TEXT NULL, ADD COLUMN answer_key_json TEXT NULL;
-- statement-break
UPDATE learning_questions SET definition_json = JSON_OBJECT('options', CAST(options_json AS JSON)), answer_key_json = JSON_OBJECT('correct_option_id', correct_option_id) WHERE question_type = 'single_choice';
-- statement-break
ALTER TABLE learning_question_answers ADD COLUMN answer_payload_json TEXT NULL, ADD COLUMN question_snapshot_json TEXT NULL, ADD COLUMN score INT NOT NULL DEFAULT 0, ADD COLUMN max_score INT NOT NULL DEFAULT 1;
-- statement-break
UPDATE learning_question_answers a JOIN learning_questions q ON q.id = a.question_id SET a.answer_payload_json = JSON_OBJECT('selected_option_id', a.selected_option_id), a.score = IF(a.is_correct, 1, 0), a.question_snapshot_json = JSON_OBJECT('question_id', q.id, 'question_type', q.question_type, 'prompt', q.prompt, 'definition', CAST(q.definition_json AS JSON), 'version', q.version);
-- statement-break
CREATE TABLE learning_question_resources (question_id VARCHAR(36) NOT NULL, resource_id VARCHAR(36) NOT NULL, position INT NOT NULL DEFAULT 0, created_at VARCHAR(40) NOT NULL, PRIMARY KEY(question_id, resource_id), FOREIGN KEY(question_id) REFERENCES learning_questions(id) ON DELETE RESTRICT, FOREIGN KEY(resource_id) REFERENCES learning_resources(id) ON DELETE RESTRICT);
-- statement-break
CREATE INDEX idx_learning_question_resources_question ON learning_question_resources(question_id, position);
