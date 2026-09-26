DROP INDEX idx_clues_location_visibility ON clues;
-- statement-break
DELETE FROM audit_events WHERE action = 'location_clue.migrated' AND entity_type = 'clue' AND entity_id IN (SELECT id FROM clues WHERE legacy_case_place_id IS NOT NULL);
-- statement-break
DELETE FROM clues WHERE legacy_case_place_id IS NOT NULL;
-- statement-break
DROP INDEX idx_clues_legacy_case_place_id ON clues;
-- statement-break
ALTER TABLE clues DROP COLUMN legacy_case_place_id;
-- statement-break
ALTER TABLE clues DROP COLUMN created_by_user_id;
-- statement-break
ALTER TABLE clues DROP COLUMN confidence;
-- statement-break
ALTER TABLE clues DROP COLUMN visibility;
-- statement-break
ALTER TABLE clues DROP COLUMN location_radius_meters;
-- statement-break
ALTER TABLE clues DROP COLUMN latitude;
-- statement-break
ALTER TABLE clues DROP COLUMN longitude;
-- statement-break
ALTER TABLE clues DROP COLUMN location_kind;
