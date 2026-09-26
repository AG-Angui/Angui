ALTER TABLE knowledge_items
    DROP FOREIGN KEY fk_knowledge_item_previous,
    DROP INDEX idx_knowledge_items_previous_version_unique;
-- statement-break
ALTER TABLE knowledge_items
    ADD CONSTRAINT fk_knowledge_item_previous
    FOREIGN KEY (previous_version_id) REFERENCES knowledge_items(id);
