UPDATE intake_question_definitions SET status = 'disabled' WHERE status = 'active';
-- statement-break
INSERT INTO intake_question_definitions (id, version, field_code, prompt, display_order, is_required, max_answer_chars, status, created_at, updated_at) VALUES
    ('intake-q-0201', 2, 'basic_information', 'Please describe the person using information your family can verify.', 1, 1, 500, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0202', 2, 'health_status', 'What health, cognitive, mobility, or medication concerns should be recorded as unconfirmed draft information?', 2, 0, 1000, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0203', 2, 'behavior_habits', 'What routines, preferences, or behaviors may help verify future leads?', 3, 0, 800, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0204', 2, 'last_seen', 'When and where was the person last seen? Include uncertainty in time, place, transport, or companions.', 4, 1, 1000, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0205', 2, 'frequent_locations', 'Which places do they commonly visit? Please avoid unrelated private addresses.', 5, 0, 800, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0206', 2, 'suspicious_motive', 'Are there any possible reasons, plans, or concerns that need careful human follow-up? Mark unknown when unsure.', 6, 0, 800, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0207', 2, 'belongings', 'What clothing, bags, phone, identification, or other belongings were they carrying?', 7, 0, 800, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0208', 2, 'transport_ability', 'How might they travel independently? Include walking, vehicle, public transport, and companion uncertainty.', 8, 0, 600, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z'),
    ('intake-q-0209', 2, 'follow_up_clues', 'Is there later information or a lead that still needs human verification?', 9, 0, 1000, 'active', '2026-07-25T00:00:00.000Z', '2026-07-25T00:00:00.000Z');
