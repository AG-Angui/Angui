ALTER TABLE learning_questions ADD COLUMN definition_json TEXT;
-- statement-break
ALTER TABLE learning_questions ADD COLUMN answer_key_json TEXT;
-- statement-break
UPDATE learning_questions SET definition_json = json_object('options', json(options_json)), answer_key_json = json_object('correct_option_id', correct_option_id) WHERE question_type = 'single_choice';
-- statement-break
ALTER TABLE learning_question_answers ADD COLUMN answer_payload_json TEXT;
-- statement-break
ALTER TABLE learning_question_answers ADD COLUMN question_snapshot_json TEXT;
-- statement-break
ALTER TABLE learning_question_answers ADD COLUMN score INTEGER NOT NULL DEFAULT 0;
-- statement-break
ALTER TABLE learning_question_answers ADD COLUMN max_score INTEGER NOT NULL DEFAULT 1;
-- statement-break
UPDATE learning_question_answers SET answer_payload_json = json_object('selected_option_id', selected_option_id), score = CASE WHEN is_correct THEN 1 ELSE 0 END, question_snapshot_json = (SELECT json_object('question_id', q.id, 'question_type', q.question_type, 'prompt', q.prompt, 'definition', json(q.definition_json), 'version', q.version) FROM learning_questions q WHERE q.id = learning_question_answers.question_id);
-- statement-break
CREATE TABLE learning_question_resources (question_id TEXT NOT NULL, resource_id TEXT NOT NULL, position INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, PRIMARY KEY(question_id, resource_id), FOREIGN KEY(question_id) REFERENCES learning_questions(id) ON DELETE RESTRICT, FOREIGN KEY(resource_id) REFERENCES learning_resources(id) ON DELETE RESTRICT);
-- statement-break
CREATE INDEX idx_learning_question_resources_question ON learning_question_resources(question_id, position);
