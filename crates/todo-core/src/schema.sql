CREATE TABLE IF NOT EXISTS tasks (
 id TEXT PRIMARY KEY, payload TEXT NOT NULL,
 due_date TEXT, planned_date TEXT, completed INTEGER NOT NULL, deleted INTEGER NOT NULL,
 series_id TEXT, occurrence_date TEXT,
 UNIQUE(series_id, occurrence_date)
);
CREATE INDEX IF NOT EXISTS tasks_due ON tasks(deleted,completed,due_date);
CREATE TABLE IF NOT EXISTS series (id TEXT PRIMARY KEY, payload TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS jobs (
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
 revision INTEGER NOT NULL, offset_days INTEGER NOT NULL, trigger_at INTEGER NOT NULL,
 status TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, retry_at INTEGER,
 batch_id TEXT, UNIQUE(task_id,revision,offset_days)
);
CREATE INDEX IF NOT EXISTS jobs_due ON jobs(status,trigger_at,retry_at);
CREATE TABLE IF NOT EXISTS inbox (id TEXT PRIMARY KEY, payload TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL);
PRAGMA user_version=1;
