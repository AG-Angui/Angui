-- Destructive rollback: m0016 refuses this script when assessments or revisions exist.
DROP TABLE IF EXISTS intake_answer_revisions;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN assessment_json;
-- statement-break
ALTER TABLE intake_sessions DROP COLUMN structured_answers_json;
