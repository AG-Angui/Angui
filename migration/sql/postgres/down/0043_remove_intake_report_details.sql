DROP TABLE IF EXISTS intake_session_photos;
-- statement-break
DELETE FROM intake_question_definitions WHERE version = 3;
-- statement-break
UPDATE intake_question_definitions SET status = 'active' WHERE version = 2;
