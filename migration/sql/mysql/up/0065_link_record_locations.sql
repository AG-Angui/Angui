ALTER TABLE locations DROP CHECK chk_locations_precision;
-- statement-break
ALTER TABLE locations ADD CONSTRAINT chk_locations_precision CHECK (location_precision IN ('unknown','approximate','exact','address','coordinates'));
-- statement-break
CREATE UNIQUE INDEX idx_locations_case_id ON locations(case_id, id);
-- statement-break
ALTER TABLE clues ADD COLUMN location_id VARCHAR(64);
-- statement-break
ALTER TABLE tasks ADD COLUMN location_id VARCHAR(64);
-- statement-break
CREATE INDEX idx_clues_location_id ON clues(location_id);
-- statement-break
CREATE INDEX idx_tasks_location_id ON tasks(location_id);
-- statement-break
CREATE INDEX idx_clues_case_location ON clues(case_id, location_id);
-- statement-break
CREATE INDEX idx_tasks_case_location ON tasks(case_id, location_id);
-- statement-break
ALTER TABLE clues ADD CONSTRAINT fk_clues_case_location FOREIGN KEY (case_id, location_id) REFERENCES locations(case_id, id);
-- statement-break
ALTER TABLE tasks ADD CONSTRAINT fk_tasks_case_location FOREIGN KEY (case_id, location_id) REFERENCES locations(case_id, id);
-- statement-break
-- Record the locations that existed before the compatibility transition.
CREATE TABLE m0065_existing_locations (id VARCHAR(64) PRIMARY KEY);
-- statement-break
INSERT INTO m0065_existing_locations (id) SELECT id FROM locations;
-- statement-break
-- Keep separate observations separate, even when their text and coordinates match.
INSERT INTO locations (id, case_id, location_text, location_precision, location_kind, longitude, latitude, location_radius_meters, created_by_user_id, created_at, updated_at)
SELECT CONCAT('clue:', c.id), c.case_id, COALESCE(c.location_text, ''), COALESCE(c.location_precision, 'unknown'), c.location_kind, c.longitude, c.latitude, c.location_radius_meters, u.id, c.created_at, c.updated_at
FROM clues c LEFT JOIN users u ON u.id = c.created_by_user_id
WHERE c.location_text IS NOT NULL OR c.location_precision IS NOT NULL OR c.location_kind IS NOT NULL OR c.longitude IS NOT NULL OR c.latitude IS NOT NULL OR c.location_radius_meters IS NOT NULL;
-- statement-break
UPDATE clues SET location_id = CONCAT('clue:', id) WHERE EXISTS (SELECT 1 FROM locations WHERE locations.id = CONCAT('clue:', clues.id));
-- statement-break
INSERT INTO locations (id, case_id, location_text, location_precision, location_kind, longitude, latitude, created_by_user_id, created_at, updated_at)
SELECT CONCAT('task:', id), case_id, area_text, CASE WHEN longitude IS NOT NULL AND latitude IS NOT NULL THEN 'exact' ELSE 'unknown' END, 'area', longitude, latitude, created_by_user_id, created_at, updated_at FROM tasks;
-- statement-break
UPDATE tasks SET location_id = CONCAT('task:', id);
