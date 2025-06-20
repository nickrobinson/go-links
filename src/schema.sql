CREATE TABLE IF NOT EXISTS links (
    id TEXT PRIMARY KEY DEFAULT (nanoid(8)),
    path TEXT NOT NULL UNIQUE,
    target TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_links_path ON links(path);
