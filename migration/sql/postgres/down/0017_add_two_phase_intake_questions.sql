-- Destructive rollback: m0017 refuses this script when v2 data has been used or modified.
DELETE FROM intake_question_definitions WHERE id IN ('intake-q-0201', 'intake-q-0202', 'intake-q-0203', 'intake-q-0204', 'intake-q-0205', 'intake-q-0206', 'intake-q-0207', 'intake-q-0208', 'intake-q-0209');
-- statement-break
UPDATE intake_question_definitions SET status = (
    SELECT previous_status FROM intake_question_definition_status_backup backup
    WHERE backup.question_id = intake_question_definitions.id
)
WHERE id IN (SELECT question_id FROM intake_question_definition_status_backup);
-- statement-break
DROP TABLE intake_question_definition_status_backup;
