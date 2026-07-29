PRAGMA foreign_keys = OFF;
-- statement-break
CREATE TABLE archive_drafts_with_review_lifecycle (
    id TEXT PRIMARY KEY NOT NULL,
    case_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'pending_review', 'published', 'rejected', 'withdrawn')),
    content TEXT NOT NULL,
    source_scope_json TEXT NOT NULL,
    deidentification_status TEXT NOT NULL CHECK (deidentification_status IN ('manual_review_required', 'deidentified', 'rejected')),
    template_version TEXT NOT NULL,
    provider_model TEXT,
    created_by_user_id TEXT NOT NULL,
    deidentified_by_user_id TEXT,
    deidentified_at TEXT,
    deidentification_reason TEXT,
    reviewed_by_user_id TEXT,
    reviewed_at TEXT,
    review_reason TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    usage_scope TEXT NOT NULL DEFAULT 'internal_archive' CHECK (usage_scope IN ('internal_archive', 'learning_resource')),
    retention_status TEXT NOT NULL DEFAULT 'retained' CHECK (retention_status IN ('retained', 'withdrawn')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((status <> 'published' OR (deidentification_status = 'deidentified' AND usage_scope = 'learning_resource' AND retention_status = 'retained')) AND (status <> 'withdrawn' OR (usage_scope = 'internal_archive' AND retention_status = 'withdrawn'))),
    FOREIGN KEY (case_id) REFERENCES cases(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (deidentified_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (reviewed_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
INSERT INTO archive_drafts_with_review_lifecycle (id, case_id, status, content, source_scope_json, deidentification_status, template_version, provider_model, created_by_user_id, created_at, updated_at)
SELECT id, case_id, status, content, source_scope_json, deidentification_status, template_version, provider_model, created_by_user_id, created_at, updated_at FROM archive_drafts;
-- statement-break
DROP TABLE archive_drafts;
-- statement-break
ALTER TABLE archive_drafts_with_review_lifecycle RENAME TO archive_drafts;
-- statement-break
CREATE INDEX idx_archive_drafts_case_created ON archive_drafts(case_id, created_at);
-- statement-break
CREATE INDEX idx_archive_drafts_status ON archive_drafts(status, updated_at);
-- statement-break
PRAGMA foreign_keys = ON;
