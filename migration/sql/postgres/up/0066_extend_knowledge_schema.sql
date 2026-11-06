ALTER TABLE knowledge_items ADD COLUMN external_id TEXT NULL;
-- statement-break
CREATE INDEX idx_knowledge_items_external ON knowledge_items(knowledge_base_id, external_id);
-- statement-break
ALTER TABLE knowledge_terms ADD COLUMN parent_id TEXT NULL REFERENCES knowledge_terms(id) ON DELETE RESTRICT;
-- statement-break
ALTER TABLE knowledge_terms ADD COLUMN description TEXT NOT NULL DEFAULT '';
-- statement-break
ALTER TABLE knowledge_import_batches ADD COLUMN source_metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb;
-- statement-break
CREATE INDEX idx_knowledge_terms_parent ON knowledge_terms(knowledge_base_id, parent_id);
