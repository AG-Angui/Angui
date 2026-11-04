DROP INDEX idx_knowledge_items_external ON knowledge_items;
-- statement-break
DROP INDEX idx_knowledge_terms_parent ON knowledge_terms;
-- statement-break
ALTER TABLE knowledge_terms DROP FOREIGN KEY fk_knowledge_term_parent;
-- statement-break
ALTER TABLE knowledge_import_batches DROP COLUMN source_metadata_json;
-- statement-break
ALTER TABLE knowledge_terms DROP COLUMN description;
-- statement-break
ALTER TABLE knowledge_terms DROP COLUMN parent_id;
-- statement-break
ALTER TABLE knowledge_items DROP COLUMN external_id;
