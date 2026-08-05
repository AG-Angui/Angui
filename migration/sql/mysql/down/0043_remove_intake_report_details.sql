DROP TABLE IF EXISTS intake_session_photos;
-- statement-break
DELETE FROM intake_question_definitions WHERE version = 3;
-- statement-break
UPDATE intake_question_definitions question
JOIN intake_question_definition_status_backup_m0043 backup ON backup.question_id = question.id
SET question.status = backup.previous_status;
-- statement-break
DROP TABLE intake_question_definition_status_backup_m0043;
