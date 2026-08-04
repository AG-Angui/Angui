CREATE TABLE learning_resources (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    content TEXT NOT NULL,
    resource_type TEXT NOT NULL CHECK (resource_type IN ('team_intro', 'manual', 'prevention', 'case_study')),
    tags_json TEXT NOT NULL,
    source_name TEXT NOT NULL,
    source_url TEXT,
    version INTEGER NOT NULL CHECK (version >= 1),
    visibility TEXT NOT NULL CHECK (visibility IN ('public', 'authenticated', 'volunteer', 'learner')),
    status TEXT NOT NULL CHECK (status IN ('published', 'withdrawn')),
    effective_at TEXT NOT NULL,
    withdrawn_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
-- statement-break
CREATE INDEX idx_learning_resources_visible ON learning_resources(status, visibility, effective_at);
-- statement-break
CREATE TABLE learning_questions (
    id TEXT PRIMARY KEY NOT NULL,
    source_resource_id TEXT NOT NULL,
    prompt TEXT NOT NULL,
    question_type TEXT NOT NULL CHECK (question_type IN ('single_choice', 'true_false', 'scenario')),
    difficulty TEXT NOT NULL CHECK (difficulty IN ('basic', 'intermediate', 'advanced')),
    tags_json TEXT NOT NULL,
    options_json TEXT NOT NULL,
    correct_option_id TEXT NOT NULL,
    explanation TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    visibility TEXT NOT NULL CHECK (visibility IN ('authenticated', 'volunteer', 'learner')),
    status TEXT NOT NULL CHECK (status IN ('published', 'withdrawn')),
    effective_at TEXT NOT NULL,
    withdrawn_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_resource_id) REFERENCES learning_resources(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_learning_questions_visible ON learning_questions(status, visibility, difficulty, effective_at);
-- statement-break
CREATE TABLE learning_question_answers (
    id TEXT PRIMARY KEY NOT NULL,
    question_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    selected_option_id TEXT NOT NULL,
    is_correct INTEGER NOT NULL CHECK (is_correct IN (0, 1)),
    question_version INTEGER NOT NULL CHECK (question_version >= 1),
    created_at TEXT NOT NULL,
    FOREIGN KEY (question_id) REFERENCES learning_questions(id) ON DELETE RESTRICT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE RESTRICT
);
-- statement-break
CREATE INDEX idx_learning_question_answers_user_created ON learning_question_answers(user_id, created_at);
