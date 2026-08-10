DROP INDEX idx_learning_resources_category ON learning_resources;
-- statement-break
ALTER TABLE learning_resources DROP FOREIGN KEY fk_learning_resources_category;
-- statement-break
ALTER TABLE learning_resources DROP COLUMN category_name;
-- statement-break
ALTER TABLE learning_resources DROP COLUMN category_id;
-- statement-break
DROP TABLE IF EXISTS learning_category_review_events;
-- statement-break
DROP TABLE IF EXISTS learning_categories;
