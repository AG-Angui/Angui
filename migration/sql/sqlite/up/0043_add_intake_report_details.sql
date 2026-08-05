CREATE TABLE intake_session_photos (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    original_filename TEXT NOT NULL,
    content_type TEXT NOT NULL CHECK (content_type IN ('image/jpeg', 'image/png')),
    byte_size INTEGER NOT NULL CHECK (byte_size > 0),
    sha256 TEXT NOT NULL,
    created_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES intake_sessions(id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_intake_session_photos_session ON intake_session_photos(session_id, created_at DESC);
-- statement-break
UPDATE intake_question_definitions SET status = 'disabled'
WHERE version = 2 AND status = 'active';
-- statement-break
INSERT INTO intake_question_definitions (id, version, field_code, prompt, display_order, is_required, max_answer_chars, status, created_at, updated_at) VALUES
    ('intake-q-0301', 3, 'basic_information', '走失者姓名、身高和特征描述', 1, 1, 700, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0302', 3, 'last_seen', '走失地点', 2, 1, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0303', 3, 'suspicious_motive', '走失原因', 3, 1, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0304', 3, 'police_report_status', '是否报警', 4, 1, 40, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0305', 3, 'family_phone', '家属电话', 5, 1, 40, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0306', 3, 'health_status', '健康、认知、行动或用药方面需要注意的情况', 6, 0, 1000, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0307', 3, 'behavior_habits', '日常习惯、偏好或行为特点', 7, 0, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0308', 3, 'frequent_locations', '常去地点（避免填写与寻人无关的隐私住址）', 8, 0, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0309', 3, 'belongings', '当时衣着、包、手机、证件或其他随身物品', 9, 0, 800, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0310', 3, 'transport_ability', '可能的独立出行方式', 10, 0, 600, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z'),
    ('intake-q-0311', 3, 'follow_up_clues', '之后获得、但仍需人工核实的信息或线索', 11, 0, 1000, 'active', '2026-08-05T00:00:00.000Z', '2026-08-05T00:00:00.000Z');
