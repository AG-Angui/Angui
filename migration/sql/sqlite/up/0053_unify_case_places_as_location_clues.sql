ALTER TABLE clues ADD COLUMN location_kind TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN longitude REAL;
-- statement-break
ALTER TABLE clues ADD COLUMN latitude REAL;
-- statement-break
ALTER TABLE clues ADD COLUMN location_radius_meters REAL;
-- statement-break
ALTER TABLE clues ADD COLUMN visibility TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN confidence TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN created_by_user_id TEXT;
-- statement-break
ALTER TABLE clues ADD COLUMN legacy_case_place_id TEXT;
-- statement-break
CREATE UNIQUE INDEX idx_clues_legacy_case_place_id ON clues(legacy_case_place_id);
-- statement-break
INSERT INTO clues (id, case_id, status, source, source_type, content, raw_record_reference, occurred_at, reported_at, confirmed_at, location_text, location_precision, location_kind, longitude, latitude, location_radius_meters, visibility, confidence, created_by_user_id, legacy_case_place_id, next_action, linked_task_reference, related_clue_id, relationship_type, review_reason, created_at, updated_at)
SELECT lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(6))), case_id,
       CASE review_status WHEN 'confirmed' THEN 'confirmed' WHEN 'rejected' THEN 'rejected' ELSE 'pending_review' END,
       source, 'location_report', name || ': ' || address, id, created_at, created_at,
       CASE review_status WHEN 'confirmed' THEN updated_at ELSE NULL END,
       address, CASE WHEN longitude IS NOT NULL AND latitude IS NOT NULL THEN 'exact' ELSE 'approximate' END,
       'point', longitude, latitude, NULL, visibility, 'unverified', created_by_user_id, id,
       NULL, NULL, NULL, NULL, NULL, created_at, updated_at
FROM case_places
WHERE NOT EXISTS (SELECT 1 FROM clues WHERE clues.legacy_case_place_id = case_places.id);
-- statement-break
INSERT INTO audit_events (id, case_id, actor, action, entity_type, entity_id, metadata_json, created_at)
SELECT lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(6))),
       p.case_id, 'system', 'location_clue.migrated', 'clue', c.id,
       json_object('legacy_case_place_id', p.id), p.updated_at
FROM case_places p JOIN clues c ON c.legacy_case_place_id = p.id
WHERE NOT EXISTS (SELECT 1 FROM audit_events a WHERE a.action = 'location_clue.migrated' AND a.entity_id = c.id);
-- statement-break
CREATE INDEX idx_clues_location_visibility ON clues(case_id, visibility, status);
