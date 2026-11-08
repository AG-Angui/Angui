DROP INDEX idx_knowledge_items_external;
-- statement-break
DROP INDEX idx_knowledge_terms_parent;
-- statement-break
ALTER TABLE knowledge_import_batches DROP COLUMN source_metadata_json;
-- statement-break
ALTER TABLE knowledge_terms DROP COLUMN description;
-- statement-break
ALTER TABLE knowledge_terms DROP COLUMN parent_id;
-- statement-break
ALTER TABLE knowledge_items DROP COLUMN external_id;
