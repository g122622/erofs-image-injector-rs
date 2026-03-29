//! Database schema

pub const SCHEMA: &str = r#"
-- Tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',

    -- Configuration
    executor_type TEXT NOT NULL,
    seeds_dir TEXT NOT NULL,
    output_dir TEXT NOT NULL,
    timeout_seconds INTEGER NOT NULL,
    max_iterations INTEGER NOT NULL DEFAULT 0,
    workers INTEGER NOT NULL DEFAULT 1,
    qemu_memory INTEGER,
    kernel_path TEXT,
    initramfs_path TEXT,
    qemu_path TEXT,
    erofsfuse_path TEXT,

    -- Statistics
    current_iteration INTEGER DEFAULT 0,
    total_crashes INTEGER DEFAULT 0,
    exec_per_sec REAL DEFAULT 0.0,

    -- Timestamps
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER,

    -- Error handling
    error_message TEXT
);

-- Index for task status queries
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);

-- Crashes table
CREATE TABLE IF NOT EXISTS crashes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    iteration INTEGER NOT NULL,
    crash_type TEXT NOT NULL,
    signal INTEGER,
    image_path TEXT NOT NULL,
    log_path TEXT,
    created_at INTEGER NOT NULL,

    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for crash queries
CREATE INDEX IF NOT EXISTS idx_crashes_task_id ON crashes(task_id);
CREATE INDEX IF NOT EXISTS idx_crashes_created_at ON crashes(created_at);
CREATE INDEX IF NOT EXISTS idx_crashes_type ON crashes(crash_type);
"#;
