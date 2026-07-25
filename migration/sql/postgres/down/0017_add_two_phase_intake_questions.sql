DELETE FROM intake_question_definitions WHERE version = 2;
-- statement-break
UPDATE intake_question_definitions SET status = 'active' WHERE version = 1;
