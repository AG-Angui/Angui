CREATE TABLE knowledge_terms (
    id TEXT PRIMARY KEY,
    knowledge_base_id TEXT NOT NULL REFERENCES knowledge_bases(id),
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('category', 'tag')),
    name VARCHAR(160) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
    created_by_user_id TEXT NOT NULL REFERENCES users(id),
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    UNIQUE (knowledge_base_id, kind, name)
);
-- statement-break
CREATE INDEX idx_knowledge_terms_base_status ON knowledge_terms(knowledge_base_id, status);
