-- Copy the authoritative location back before removing references.
UPDATE clues SET
    location_text = (SELECT CASE WHEN l.location_text = '' AND clues.location_text IS NULL THEN NULL ELSE l.location_text END FROM locations l WHERE l.id = clues.location_id),
    location_precision = (SELECT CASE WHEN l.location_precision = 'unknown' AND clues.location_precision IS NULL THEN NULL ELSE l.location_precision END FROM locations l WHERE l.id = clues.location_id),
    location_kind = (SELECT l.location_kind FROM locations l WHERE l.id = clues.location_id),
    longitude = (SELECT l.longitude FROM locations l WHERE l.id = clues.location_id),
    latitude = (SELECT l.latitude FROM locations l WHERE l.id = clues.location_id),
    location_radius_meters = (SELECT l.location_radius_meters FROM locations l WHERE l.id = clues.location_id)
WHERE location_id IS NOT NULL;
-- statement-break
UPDATE tasks SET
    area_text = (SELECT l.location_text FROM locations l WHERE l.id = tasks.location_id),
    longitude = (SELECT l.longitude FROM locations l WHERE l.id = tasks.location_id),
    latitude = (SELECT l.latitude FROM locations l WHERE l.id = tasks.location_id)
WHERE location_id IS NOT NULL;
-- statement-break
ALTER TABLE tasks DROP FOREIGN KEY fk_tasks_case_location;
-- statement-break
ALTER TABLE clues DROP FOREIGN KEY fk_clues_case_location;
-- statement-break
DROP INDEX idx_tasks_location_id ON tasks;
-- statement-break
DROP INDEX idx_clues_location_id ON clues;
-- statement-break
ALTER TABLE tasks DROP COLUMN location_id;
-- statement-break
ALTER TABLE clues DROP COLUMN location_id;
-- statement-break
DELETE FROM locations WHERE id LIKE 'clue:%' OR id LIKE 'task:%';
-- statement-break
DROP INDEX idx_locations_case_id ON locations;
-- Precision widening is backward compatible and remains until m0064 is rolled back.

-- statement-break
DROP TABLE m0065_existing_locations;
