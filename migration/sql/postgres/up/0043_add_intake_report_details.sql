CREATE TABLE intake_session_photos (
    id VARCHAR(36) PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL REFERENCES intake_sessions(id) ON UPDATE CASCADE ON DELETE CASCADE,
    storage_key VARCHAR(255) NOT NULL UNIQUE,
    original_filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(80) NOT NULL CHECK (content_type IN ('image/jpeg', 'image/png')),
    byte_size BIGINT NOT NULL CHECK (byte_size > 0),
    sha256 VARCHAR(64) NOT NULL,
    created_by_user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    created_at VARCHAR(40) NOT NULL
);
-- statement-break
CREATE INDEX idx_intake_session_photos_session ON intake_session_photos(session_id, created_at DESC);
-- statement-break
CREATE TABLE intake_question_definition_status_backup_m0043 (
    question_id VARCHAR(36) PRIMARY KEY REFERENCES intake_question_definitions(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    previous_status VARCHAR(16) NOT NULL CHECK (previous_status = 'active')
);
-- statement-break
INSERT INTO intake_question_definition_status_backup_m0043 (question_id, previous_status)
SELECT id, status FROM intake_question_definitions WHERE version = 2 AND status = 'active';
-- statement-break
UPDATE intake_question_definitions SET status = 'disabled' WHERE version = 2 AND status = 'active';
-- statement-break
INSERT INTO intake_question_definitions (id, version, field_code, prompt, display_order, is_required, max_answer_chars, status, created_at, updated_at) VALUES
('intake-q-0301', 3, 'basic_information', '走失者姓名、身高和特征描述', 1, TRUE, 700, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0302', 3, 'last_seen', '走失地点', 2, TRUE, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0303', 3, 'suspicious_motive', '走失原因', 3, TRUE, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0304', 3, 'police_report_status', '是否报警', 4, TRUE, 40, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0305', 3, 'family_phone', '家属电话', 5, TRUE, 40, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0306', 3, 'health_status', '健康、认知、行动或用药方面需要注意的情况', 6, FALSE, 1000, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0307', 3, 'behavior_habits', '日常习惯、偏好或行为特点', 7, FALSE, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0308', 3, 'frequent_locations', '常去地点（避免填写与寻人无关的隐私住址）', 8, FALSE, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0309', 3, 'belongings', '当时衣着、包、手机、证件或其他随身物品', 9, FALSE, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0310', 3, 'transport_ability', '可能的独立出行方式', 10, FALSE, 600, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'), ('intake-q-0311', 3, 'follow_up_clues', '之后获得、但仍需人工核实的信息或线索', 11, FALSE, 1000, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z');
