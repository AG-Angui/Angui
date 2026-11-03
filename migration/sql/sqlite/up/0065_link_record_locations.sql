-- Replace only the new locations table to extend its precision constraint.
-- No existing table references locations before this migration.
CREATE TABLE locations_with_exact (
    id TEXT PRIMARY KEY,
    case_id TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    location_text TEXT NOT NULL,
    location_precision TEXT NOT NULL DEFAULT 'unknown' CHECK (location_precision IN ('unknown','approximate','exact','address','coordinates')),
    location_kind TEXT,
    longitude REAL CHECK (longitude BETWEEN -180 AND 180),
    latitude REAL CHECK (latitude BETWEEN -90 AND 90),
    location_radius_meters REAL CHECK (location_radius_meters >= 0),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','merged','retired')),
    merged_into_id TEXT REFERENCES locations_with_exact(id) ON DELETE SET NULL,
    created_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
INSERT INTO locations_with_exact SELECT * FROM locations;
-- statement-break
DROP TABLE locations;
-- statement-break
ALTER TABLE locations_with_exact RENAME TO locations;
-- statement-break
CREATE INDEX idx_locations_case_status ON locations(case_id, status, updated_at);
-- statement-break
CREATE UNIQUE INDEX idx_locations_case_id ON locations(case_id, id);
-- statement-break
ALTER TABLE clues ADD COLUMN location_id TEXT REFERENCES locations(id);
-- statement-break
ALTER TABLE tasks ADD COLUMN location_id TEXT REFERENCES locations(id);
-- statement-break
CREATE INDEX idx_clues_location_id ON clues(location_id);
-- statement-break
CREATE INDEX idx_tasks_location_id ON tasks(location_id);
-- statement-break
CREATE TRIGGER clues_location_case_insert BEFORE INSERT ON clues
WHEN NEW.location_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM locations WHERE id = NEW.location_id AND case_id = NEW.case_id)
BEGIN SELECT RAISE(ABORT, 'clue location must belong to the same case'); END;
-- statement-break
CREATE TRIGGER clues_location_case_update BEFORE UPDATE OF location_id, case_id ON clues
WHEN NEW.location_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM locations WHERE id = NEW.location_id AND case_id = NEW.case_id)
BEGIN SELECT RAISE(ABORT, 'clue location must belong to the same case'); END;
-- statement-break
CREATE TRIGGER tasks_location_case_insert BEFORE INSERT ON tasks
WHEN NEW.location_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM locations WHERE id = NEW.location_id AND case_id = NEW.case_id)
BEGIN SELECT RAISE(ABORT, 'task location must belong to the same case'); END;
-- statement-break
CREATE TRIGGER tasks_location_case_update BEFORE UPDATE OF location_id, case_id ON tasks
WHEN NEW.location_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM locations WHERE id = NEW.location_id AND case_id = NEW.case_id)
BEGIN SELECT RAISE(ABORT, 'task location must belong to the same case'); END;
-- statement-break
CREATE TRIGGER locations_case_update BEFORE UPDATE OF case_id ON locations
WHEN OLD.case_id <> NEW.case_id AND (EXISTS (SELECT 1 FROM clues WHERE location_id = OLD.id) OR EXISTS (SELECT 1 FROM tasks WHERE location_id = OLD.id))
BEGIN SELECT RAISE(ABORT, 'referenced location cannot change case'); END;
-- statement-break
-- Record the locations that existed before the compatibility transition.
CREATE TABLE m0065_existing_locations (id TEXT PRIMARY KEY);
-- statement-break
INSERT INTO m0065_existing_locations (id) SELECT id FROM locations;
-- statement-break
-- Keep separate observations separate, even when their text and coordinates match.
INSERT INTO locations (id, case_id, location_text, location_precision, location_kind, longitude, latitude, location_radius_meters, created_by_user_id, created_at, updated_at)
SELECT 'clue:' || c.id, c.case_id, COALESCE(c.location_text, ''), COALESCE(c.location_precision, 'unknown'), c.location_kind, c.longitude, c.latitude, c.location_radius_meters, u.id, c.created_at, c.updated_at
FROM clues c LEFT JOIN users u ON u.id = c.created_by_user_id
WHERE c.location_text IS NOT NULL OR c.location_precision IS NOT NULL OR c.location_kind IS NOT NULL OR c.longitude IS NOT NULL OR c.latitude IS NOT NULL OR c.location_radius_meters IS NOT NULL;
-- statement-break
UPDATE clues SET location_id = 'clue:' || id WHERE EXISTS (SELECT 1 FROM locations WHERE locations.id = 'clue:' || clues.id);
-- statement-break
INSERT INTO locations (id, case_id, location_text, location_precision, location_kind, longitude, latitude, created_by_user_id, created_at, updated_at)
SELECT 'task:' || id, case_id, area_text, CASE WHEN longitude IS NOT NULL AND latitude IS NOT NULL THEN 'exact' ELSE 'unknown' END, 'area', longitude, latitude, created_by_user_id, created_at, updated_at FROM tasks;
-- statement-break
UPDATE tasks SET location_id = 'task:' || id;
