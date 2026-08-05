DROP TABLE IF EXISTS intake_session_photos;
-- statement-break
DELETE FROM intake_question_definitions WHERE version = 3;
-- statement-break
UPDATE intake_question_definitions question
SET status = backup.previous_status
FROM intake_question_definition_status_backup_m0043 backup
WHERE backup.question_id = question.id;
-- statement-break
DROP TABLE intake_question_definition_status_backup_m0043;
