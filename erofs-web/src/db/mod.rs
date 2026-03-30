//! Database layer for the web console

mod schema;

use std::path::Path;
use std::sync::Arc;

use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::types::*;

pub use schema::SCHEMA;

/// Database wrapper with async support
#[derive(Debug, Clone)]
pub struct Database {
    inner: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let path = path.as_ref();
        info!("Opening database at {:?}", path);

        let conn = Connection::open(path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Create tables
        conn.execute_batch(SCHEMA)?;
        Self::migrate_schema(&conn)?;

        debug!("Database initialized successfully");

        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(SCHEMA)?;
        Self::migrate_schema(&conn)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
        })
    }

    /// Apply schema migrations for older databases.
    fn migrate_schema(conn: &Connection) -> SqliteResult<()> {
        Self::add_column_if_missing(conn, "tasks", "qemu_path", "TEXT")?;
        Self::add_column_if_missing(conn, "tasks", "erofsfuse_path", "TEXT")?;
        Self::add_column_if_missing(conn, "tasks", "strategy_id", "INTEGER")?;
        Self::add_column_if_missing(conn, "tasks", "kernel_version", "TEXT")?;
        Self::add_column_if_missing(conn, "tasks", "erofs_version", "TEXT")?;
        Ok(())
    }

    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        column_def: &str,
    ) -> SqliteResult<()> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;

        if columns.iter().any(|c| c == column) {
            return Ok(());
        }

        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, column_def),
            [],
        )?;

        Ok(())
    }

    // ========== Task operations ==========

    /// Create a new task
    pub async fn create_task(&self, config: &TaskConfig) -> SqliteResult<i64> {
        let conn = self.inner.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT INTO tasks (
                name, status, executor_type, seeds_dir, output_dir,
                timeout_seconds, max_iterations, workers,
                qemu_memory, kernel_path, initramfs_path,
                qemu_path, erofsfuse_path, strategy_id,
                current_iteration, total_crashes, exec_per_sec,
                created_at
            ) VALUES (?1, 'pending', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, 0, 0.0, ?14)
            "#,
            params![
                config.name,
                config.executor_type.to_string(),
                config.seeds_dir,
                config.output_dir,
                config.timeout_seconds as i64,
                config.max_iterations as i64,
                config.workers as i64,
                config.qemu_memory.map(|m| m as i64),
                config.kernel_path,
                config.initramfs_path,
                config.qemu_path,
                config.erofsfuse_path,
                config.strategy_id,
                now,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a task by ID
    pub async fn get_task(&self, id: i64) -> SqliteResult<Option<Task>> {
        let conn = self.inner.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, status, executor_type, seeds_dir, output_dir,
                   timeout_seconds, max_iterations, workers,
                   qemu_memory, kernel_path, initramfs_path,
                   qemu_path, erofsfuse_path, strategy_id,
                   current_iteration, total_crashes, exec_per_sec,
                   kernel_version, erofs_version,
                   created_at, started_at, finished_at, error_message
            FROM tasks WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get::<_, String>(2)?.parse().unwrap_or(TaskStatus::Pending),
                executor_type: row.get::<_, String>(3)?.parse().unwrap_or(ExecutorType::Erofsfuse),
                seeds_dir: row.get(4)?,
                output_dir: row.get(5)?,
                timeout_seconds: row.get::<_, i64>(6)? as u64,
                max_iterations: row.get::<_, i64>(7)? as u64,
                workers: row.get::<_, i64>(8)? as usize,
                qemu_memory: row.get::<_, Option<i64>>(9)?.map(|m| m as usize),
                kernel_path: row.get(10)?,
                initramfs_path: row.get(11)?,
                qemu_path: row.get(12)?,
                erofsfuse_path: row.get(13)?,
                strategy_id: row.get(14)?,
                current_iteration: row.get::<_, i64>(15)? as u64,
                total_crashes: row.get::<_, i64>(16)? as u64,
                exec_per_sec: row.get(17)?,
                kernel_version: row.get(18)?,
                erofs_version: row.get(19)?,
                created_at: chrono::DateTime::from_timestamp(row.get::<_, i64>(20)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
                started_at: row.get::<_, Option<i64>>(21)?.and_then(|t| {
                    chrono::DateTime::from_timestamp(t, 0)
                }),
                finished_at: row.get::<_, Option<i64>>(22)?.and_then(|t| {
                    chrono::DateTime::from_timestamp(t, 0)
                }),
                error_message: row.get(23)?,
            })
        });

        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> SqliteResult<Vec<Task>> {
        let conn = self.inner.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, status, executor_type, seeds_dir, output_dir,
                   timeout_seconds, max_iterations, workers,
                   qemu_memory, kernel_path, initramfs_path,
                   qemu_path, erofsfuse_path, strategy_id,
                   current_iteration, total_crashes, exec_per_sec,
                   kernel_version, erofs_version,
                   created_at, started_at, finished_at, error_message
            FROM tasks ORDER BY created_at DESC
            "#,
        )?;

        let tasks = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get::<_, String>(2)?.parse().unwrap_or(TaskStatus::Pending),
                executor_type: row.get::<_, String>(3)?.parse().unwrap_or(ExecutorType::Erofsfuse),
                seeds_dir: row.get(4)?,
                output_dir: row.get(5)?,
                timeout_seconds: row.get::<_, i64>(6)? as u64,
                max_iterations: row.get::<_, i64>(7)? as u64,
                workers: row.get::<_, i64>(8)? as usize,
                qemu_memory: row.get::<_, Option<i64>>(9)?.map(|m| m as usize),
                kernel_path: row.get(10)?,
                initramfs_path: row.get(11)?,
                qemu_path: row.get(12)?,
                erofsfuse_path: row.get(13)?,
                strategy_id: row.get(14)?,
                current_iteration: row.get::<_, i64>(15)? as u64,
                total_crashes: row.get::<_, i64>(16)? as u64,
                exec_per_sec: row.get(17)?,
                kernel_version: row.get(18)?,
                erofs_version: row.get(19)?,
                created_at: chrono::DateTime::from_timestamp(row.get::<_, i64>(20)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
                started_at: row.get::<_, Option<i64>>(21)?.and_then(|t| {
                    chrono::DateTime::from_timestamp(t, 0)
                }),
                finished_at: row.get::<_, Option<i64>>(22)?.and_then(|t| {
                    chrono::DateTime::from_timestamp(t, 0)
                }),
                error_message: row.get(23)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Update task status
    pub async fn update_task_status(&self, id: i64, status: TaskStatus) -> SqliteResult<()> {
        let conn = self.inner.lock().await;
        let now = chrono::Utc::now().timestamp();

        let (started_at, finished_at) = match status {
            TaskStatus::Running => (Some(now), None),
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => (None, Some(now)),
            _ => (None, None),
        };

        if let Some(started) = started_at {
            conn.execute(
                "UPDATE tasks SET status = ?1, started_at = ?2 WHERE id = ?3",
                params![status.to_string(), started, id],
            )?;
        } else if let Some(finished) = finished_at {
            conn.execute(
                "UPDATE tasks SET status = ?1, finished_at = ?2 WHERE id = ?3",
                params![status.to_string(), finished, id],
            )?;
        } else {
            conn.execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                params![status.to_string(), id],
            )?;
        }

        Ok(())
    }

    /// Update task progress
    pub async fn update_task_progress(
        &self,
        id: i64,
        iteration: u64,
        crashes: u64,
        speed: f64,
    ) -> SqliteResult<()> {
        let conn = self.inner.lock().await;

        conn.execute(
            "UPDATE tasks SET current_iteration = ?1, total_crashes = ?2, exec_per_sec = ?3 WHERE id = ?4",
            params![iteration as i64, crashes as i64, speed, id],
        )?;

        Ok(())
    }

    /// Update task version information
    pub async fn update_task_versions(
        &self,
        id: i64,
        kernel_version: Option<&str>,
        erofs_version: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.inner.lock().await;

        conn.execute(
            "UPDATE tasks SET kernel_version = ?1, erofs_version = ?2 WHERE id = ?3",
            params![kernel_version, erofs_version, id],
        )?;

        Ok(())
    }

    /// Set task error message
    pub async fn set_task_error(&self, id: i64, error: &str) -> SqliteResult<()> {
        let conn = self.inner.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE tasks SET status = 'failed', error_message = ?1, finished_at = ?2 WHERE id = ?3",
            params![error, now, id],
        )?;

        Ok(())
    }

    /// Delete a task
    pub async fn delete_task(&self, id: i64) -> SqliteResult<()> {
        let conn = self.inner.lock().await;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ========== Crash operations ==========

    /// Create a crash record
    pub async fn create_crash(
        &self,
        task_id: i64,
        iteration: u64,
        crash_type: CrashType,
        signal: Option<i32>,
        image_path: &str,
        log_path: Option<&str>,
    ) -> SqliteResult<i64> {
        info!("[DB] create_crash: task_id={}, iteration={}, type={:?}, path={}", task_id, iteration, crash_type, image_path);
        let conn = self.inner.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT INTO crashes (task_id, iteration, crash_type, signal, image_path, log_path, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                task_id,
                iteration as i64,
                crash_type.to_string(),
                signal,
                image_path,
                log_path,
                now,
            ],
        )?;

        let id = conn.last_insert_rowid();
        info!("[DB] create_crash: inserted crash id={}", id);
        Ok(id)
    }

    /// Get a crash by ID
    pub async fn get_crash(&self, id: i64) -> SqliteResult<Option<Crash>> {
        let conn = self.inner.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, task_id, iteration, crash_type, signal, image_path, log_path, created_at
            FROM crashes WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(Crash {
                id: row.get(0)?,
                task_id: row.get(1)?,
                iteration: row.get::<_, i64>(2)? as u64,
                crash_type: row.get::<_, String>(3)?.parse().unwrap_or(CrashType::Signal),
                signal: row.get(4)?,
                image_path: row.get(5)?,
                log_path: row.get(6)?,
                created_at: chrono::DateTime::from_timestamp(row.get::<_, i64>(7)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        });

        match result {
            Ok(crash) => Ok(Some(crash)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// List crashes with filter
    pub async fn list_crashes(&self, filter: &CrashFilter) -> SqliteResult<Vec<Crash>> {
        let conn = self.inner.lock().await;

        let mut sql = String::from(
            "SELECT id, task_id, iteration, crash_type, signal, image_path, log_path, created_at FROM crashes WHERE 1=1"
        );
        let mut bind_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(task_id) = filter.task_id {
            sql.push_str(" AND task_id = ?");
            bind_params.push(Box::new(task_id));
        }

        if let Some(crash_type) = &filter.crash_type {
            sql.push_str(" AND crash_type = ?");
            bind_params.push(Box::new(crash_type.to_string()));
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        info!("[DB] list_crashes SQL: {}", sql);
        let params: Vec<&dyn rusqlite::ToSql> = bind_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let crashes = stmt.query_map(params.as_slice(), |row| {
            Ok(Crash {
                id: row.get(0)?,
                task_id: row.get(1)?,
                iteration: row.get::<_, i64>(2)? as u64,
                crash_type: row.get::<_, String>(3)?.parse().unwrap_or(CrashType::Signal),
                signal: row.get(4)?,
                image_path: row.get(5)?,
                log_path: row.get(6)?,
                created_at: chrono::DateTime::from_timestamp(row.get::<_, i64>(7)?, 0)
                    .unwrap_or_else(chrono::Utc::now),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        info!("[DB] list_crashes returning {} crashes", crashes.len());
        Ok(crashes)
    }

    /// Get task statistics
    pub async fn get_stats(&self) -> SqliteResult<TaskStats> {
        let conn = self.inner.lock().await;

        let total: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
        let running: i64 = conn.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'running'", [], |row| row.get(0))?;
        let pending: i64 = conn.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'pending'", [], |row| row.get(0))?;
        let completed: i64 = conn.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'completed'", [], |row| row.get(0))?;
        let failed: i64 = conn.query_row("SELECT COUNT(*) FROM tasks WHERE status = 'failed'", [], |row| row.get(0))?;
        let total_crashes: i64 = conn.query_row("SELECT COALESCE(SUM(total_crashes), 0) FROM tasks", [], |row| row.get(0))?;
        let total_iterations: i64 = conn.query_row("SELECT COALESCE(SUM(current_iteration), 0) FROM tasks", [], |row| row.get(0))?;

        Ok(TaskStats {
            total: total as u64,
            running: running as u64,
            pending: pending as u64,
            completed: completed as u64,
            failed: failed as u64,
            total_crashes: total_crashes as u64,
            total_iterations: total_iterations as u64,
        })
    }

    // ========== Mutator Statistics operations ==========

    /// Update or create mutator statistics for a task
    pub async fn update_mutator_stats(
        &self,
        task_id: i64,
        mutator: &str,
        executions: u64,
        crashes: u64,
        current_weight: u32,
        original_weight: u32,
    ) -> SqliteResult<()> {
        let conn = self.inner.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT INTO mutator_stats (task_id, mutator, executions, crashes, current_weight, original_weight, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(task_id, mutator) DO UPDATE SET
                executions = excluded.executions,
                crashes = excluded.crashes,
                current_weight = excluded.current_weight,
                original_weight = excluded.original_weight,
                updated_at = excluded.updated_at
            "#,
            params![task_id, mutator, executions as i64, crashes as i64, current_weight as i64, original_weight as i64, now],
        )?;

        Ok(())
    }

    /// Increment mutator statistics
    pub async fn increment_mutator_stats(
        &self,
        task_id: i64,
        mutator: &str,
        executions_delta: u64,
        crashes_delta: u64,
    ) -> SqliteResult<()> {
        let conn = self.inner.lock().await;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT INTO mutator_stats (task_id, mutator, executions, crashes, current_weight, original_weight, updated_at)
            VALUES (?1, ?2, ?3, ?4, 0, 0, ?5)
            ON CONFLICT(task_id, mutator) DO UPDATE SET
                executions = executions + ?3,
                crashes = crashes + ?4,
                updated_at = excluded.updated_at
            "#,
            params![task_id, mutator, executions_delta as i64, crashes_delta as i64, now],
        )?;

        Ok(())
    }

    /// Get mutator statistics for a task
    pub async fn get_mutator_stats(&self, task_id: i64) -> SqliteResult<Vec<MutatorStatsRecord>> {
        let conn = self.inner.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT mutator, executions, crashes, current_weight, original_weight
            FROM mutator_stats WHERE task_id = ?1
            "#,
        )?;

        let stats = stmt.query_map(params![task_id], |row| {
            Ok(MutatorStatsRecord {
                mutator: row.get(0)?,
                executions: row.get::<_, i64>(1)? as u64,
                crashes: row.get::<_, i64>(2)? as u64,
                current_weight: row.get::<_, i64>(3)? as u32,
                original_weight: row.get::<_, i64>(4)? as u32,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
    }

    /// Delete mutator statistics for a task
    pub async fn delete_mutator_stats(&self, task_id: i64) -> SqliteResult<()> {
        let conn = self.inner.lock().await;
        conn.execute("DELETE FROM mutator_stats WHERE task_id = ?1", params![task_id])?;
        Ok(())
    }
}

/// Mutator statistics record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutatorStatsRecord {
    /// Mutator type name
    pub mutator: String,
    /// Total executions
    pub executions: u64,
    /// Total crashes
    pub crashes: u64,
    /// Current weight (may change with adaptive)
    pub current_weight: u32,
    /// Original weight
    pub original_weight: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_crud() {
        let db = Database::in_memory().unwrap();

        // Create task
        let config = TaskConfig {
            name: "test-task".to_string(),
            executor_type: ExecutorType::Qemu,
            seeds_dir: "./seeds".to_string(),
            output_dir: "./crashes".to_string(),
            timeout_seconds: 120,
            max_iterations: 1000,
            workers: 2,
            qemu_memory: Some(512),
            kernel_path: Some("./kernel_build/bzImage".to_string()),
            initramfs_path: Some("./kernel_build/rootfs.cpio.gz".to_string()),
            ..Default::default()
        };

        let task_id = db.create_task(&config).await.unwrap();
        assert!(task_id > 0);

        // Get task
        let task = db.get_task(task_id).await.unwrap().unwrap();
        assert_eq!(task.name, "test-task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.executor_type, ExecutorType::Qemu);

        // Update status
        db.update_task_status(task_id, TaskStatus::Running).await.unwrap();
        let task = db.get_task(task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Running);

        // Update progress
        db.update_task_progress(task_id, 100, 5, 15.5).await.unwrap();
        let task = db.get_task(task_id).await.unwrap().unwrap();
        assert_eq!(task.current_iteration, 100);
        assert_eq!(task.total_crashes, 5);

        // Create crash
        let crash_id = db.create_crash(
            task_id,
            50,
            CrashType::KernelPanic,
            None,
            "/crashes/crash-001.erofs",
            Some("/crashes/crash-001.log"),
        ).await.unwrap();
        assert!(crash_id > 0);

        // Get crash
        let crash = db.get_crash(crash_id).await.unwrap().unwrap();
        assert_eq!(crash.task_id, task_id);
        assert_eq!(crash.crash_type, CrashType::KernelPanic);

        // List crashes
        let crashes = db.list_crashes(&CrashFilter { task_id: Some(task_id), ..Default::default() }).await.unwrap();
        assert_eq!(crashes.len(), 1);

        // Get stats
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.running, 1);
    }
}
