ALTER TABLE intake_sessions ADD COLUMN question_set_version INT NOT NULL DEFAULT 1;
-- statement-break
CREATE TABLE intake_question_definitions (
    id VARCHAR(36) PRIMARY KEY,
    version INT NOT NULL,
    field_code VARCHAR(64) NOT NULL,
    prompt TEXT NOT NULL,
    display_order INT NOT NULL,
    is_required BOOLEAN NOT NULL,
    max_answer_chars INT NOT NULL,
    status VARCHAR(16) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT chk_intake_question_definitions_version CHECK (version > 0),
    CONSTRAINT chk_intake_question_definitions_display_order CHECK (display_order > 0),
    CONSTRAINT chk_intake_question_definitions_answer_limit CHECK (max_answer_chars > 0),
    CONSTRAINT chk_intake_question_definitions_status CHECK (status IN ('active', 'disabled')),
    UNIQUE (version, field_code),
    UNIQUE (version, display_order)
) ENGINE=InnoDB;
-- statement-break
CREATE INDEX idx_intake_question_definitions_active ON intake_question_definitions(status, version, display_order);
-- statement-break
INSERT INTO intake_question_definitions (id, version, field_code, prompt, display_order, is_required, max_answer_chars, status, created_at, updated_at) VALUES
    ('intake-q-0001', 1, 'basic_information', 'Please describe the person in a way your family can verify, such as their name or a safe identifying description.', 1, TRUE, 500, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0002', 1, 'health_status', 'Are there health, cognitive, mobility, or medication concerns responders should know? Share only what is necessary.', 2, FALSE, 1000, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0003', 1, 'behavior_habits', 'What routines, preferences, or behaviors may help family members recognize useful leads?', 3, FALSE, 800, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0004', 1, 'last_seen', 'When and where was the person last seen? Include uncertainty if the time or place is not exact.', 4, TRUE, 800, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0005', 1, 'frequent_locations', 'Which places do they commonly visit? Please avoid unrelated private addresses.', 5, FALSE, 800, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0006', 1, 'belongings', 'What clothing, bags, phone, identification, or other belongings were they carrying?', 6, FALSE, 800, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0007', 1, 'transport_ability', 'How might they travel independently, for example walking, public transport, or a familiar route?', 7, FALSE, 600, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z'),
    ('intake-q-0008', 1, 'follow_up_clues', 'Is there any later information or lead that still needs human verification?', 8, FALSE, 1000, 'active', '2026-07-24T00:00:00.000Z', '2026-07-24T00:00:00.000Z');
