CREATE TABLE knowledge_terms (
    id VARCHAR(191) PRIMARY KEY,
    knowledge_base_id VARCHAR(191) NOT NULL,
    kind VARCHAR(16) NOT NULL,
    name VARCHAR(160) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    created_by_user_id VARCHAR(191) NOT NULL,
    created_at VARCHAR(40) NOT NULL,
    updated_at VARCHAR(40) NOT NULL,
    CONSTRAINT fk_knowledge_terms_base FOREIGN KEY (knowledge_base_id) REFERENCES knowledge_bases(id),
    CONSTRAINT fk_knowledge_terms_creator FOREIGN KEY (created_by_user_id) REFERENCES users(id),
    CONSTRAINT chk_knowledge_terms_kind CHECK (kind IN ('category', 'tag')),
    CONSTRAINT chk_knowledge_terms_status CHECK (status IN ('active', 'archived')),
    UNIQUE KEY idx_knowledge_terms_unique (knowledge_base_id, kind, name),
    INDEX idx_knowledge_terms_base_status (knowledge_base_id, status)
) ENGINE=InnoDB;
