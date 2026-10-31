CREATE TABLE knowledge_terms (
    id TEXT PRIMARY KEY NOT NULL,
    knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id),
    kind TEXT NOT NULL CHECK (kind IN ('category', 'tag')),
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
    created_by_user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (knowledge_base_id, kind, name)
);
-- statement-break
CREATE INDEX idx_knowledge_terms_base_status ON knowledge_terms(knowledge_base_id, status);
