DROP INDEX idx_clues_location_visibility;
-- statement-break
DELETE FROM clues WHERE legacy_case_place_id IS NOT NULL;
-- statement-break
DROP INDEX idx_clues_legacy_case_place_id;
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
