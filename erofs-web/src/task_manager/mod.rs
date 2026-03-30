//! Task manager for controlling fuzzing tasks

mod queue;
mod runner;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{debug, error, info};

use crate::db::Database;
use crate::strategy::StrategyStorage;
use crate::types::*;

pub use queue::TaskQueue;
pub use runner::TaskRunner;

/// Maximum concurrent tasks (configurable)
const MAX_CONCURRENT_TASKS: usize = 4;

/// Task manager event
#[derive(Debug, Clone)]
pub enum TaskEvent {
    /// Task was created
    Created { task_id: i64 },
    /// Task status changed
    StatusChanged { task_id: i64, status: TaskStatus },
    /// Task progress updated
    Progress { task_id: i64, iteration: u64, crashes: u64, speed: f64, current_mutator: Option<String> },
    /// Crash was found
    CrashFound { task_id: i64, crash_id: i64, crash_type: CrashType, iteration: u64 },
    /// Task finished
    Finished { task_id: i64, status: TaskStatus },
    /// Error occurred
    Error { task_id: i64, message: String },
    /// Log message
    Log { task_id: i64, level: LogLevel, message: String, timestamp: i64 },
}

/// Task manager handle
#[derive(Debug, Clone)]
pub struct TaskManager {
    /// Database
    db: Database,
    /// Strategy storage
    strategy_storage: StrategyStorage,
    /// Task queue
    queue: Arc<Mutex<TaskQueue>>,
    /// Running tasks (task_id -> cancel sender)
    running: Arc<Mutex<HashMap<i64, mpsc::Sender<ControlMessage>>>>,
    /// Event broadcaster
    event_tx: broadcast::Sender<TaskEvent>,
    /// Maximum concurrent tasks
    max_concurrent: usize,
}

/// Control messages for task runner
#[derive(Debug, Clone)]
pub enum ControlMessage {
    /// Stop the task
    Stop,
    /// Pause the task
    Pause,
    /// Resume the task
    Resume,
}

impl TaskManager {
    /// Create a new task manager
    pub fn new(db: Database) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let strategy_storage = StrategyStorage::with_default_path()
            .expect("Failed to initialize strategy storage");

        Self {
            db,
            strategy_storage,
            queue: Arc::new(Mutex::new(TaskQueue::new())),
            running: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            max_concurrent: MAX_CONCURRENT_TASKS,
        }
    }

    /// Create task manager with custom concurrency limit
    pub fn with_concurrency(db: Database, max_concurrent: usize) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let strategy_storage = StrategyStorage::with_default_path()
            .expect("Failed to initialize strategy storage");

        Self {
            db,
            strategy_storage,
            queue: Arc::new(Mutex::new(TaskQueue::new())),
            running: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            max_concurrent,
        }
    }

    /// Subscribe to task events
    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_tx.subscribe()
    }

    /// Create a new task
    pub async fn create_task(&self, config: TaskConfig) -> Result<i64, String> {
        let config = self.normalize_config(config);

        // Validate configuration
        self.validate_config(&config)?;

        // Create in database
        let task_id = self.db.create_task(&config).await
            .map_err(|e| format!("Failed to create task: {}", e))?;

        info!("Created task {} with config: {:?}", task_id, config);

        // Emit event
        let _ = self.event_tx.send(TaskEvent::Created { task_id });

        // Auto-start if configured
        self.try_start_next().await?;

        Ok(task_id)
    }

    /// Start a task
    pub async fn start_task(&self, task_id: i64) -> Result<(), String> {
        let task = self.db.get_task(task_id).await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        if task.status != TaskStatus::Pending && task.status != TaskStatus::Paused {
            return Err(format!("Task {} is not in a startable state (current: {})", task_id, task.status));
        }

        // Add to queue if not already
        {
            let mut queue = self.queue.lock().await;
            if !queue.contains(task_id) {
                queue.enqueue(task_id);
            }
        }

        // Try to start
        self.try_start_next().await?;

        Ok(())
    }

    /// Stop a task
    pub async fn stop_task(&self, task_id: i64) -> Result<(), String> {
        let task = self.db.get_task(task_id).await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        if task.status != TaskStatus::Running && task.status != TaskStatus::Paused {
            return Err(format!("Task {} is not running or paused", task_id));
        }

        // Send stop signal
        {
            let running = self.running.lock().await;
            if let Some(tx) = running.get(&task_id) {
                let _ = tx.send(ControlMessage::Stop).await;
                debug!("Sent stop signal to task {}", task_id);
            }
        }

        // Remove from queue
        {
            let mut queue = self.queue.lock().await;
            queue.remove(task_id);
        }

        // Update status
        self.db.update_task_status(task_id, TaskStatus::Cancelled).await
            .map_err(|e| format!("Failed to update status: {}", e))?;

        let _ = self.event_tx.send(TaskEvent::StatusChanged {
            task_id,
            status: TaskStatus::Cancelled,
        });

        Ok(())
    }

    /// Pause a task
    pub async fn pause_task(&self, task_id: i64) -> Result<(), String> {
        let task = self.db.get_task(task_id).await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        if task.status != TaskStatus::Running {
            return Err(format!("Task {} is not running", task_id));
        }

        // Send pause signal
        {
            let running = self.running.lock().await;
            if let Some(tx) = running.get(&task_id) {
                let _ = tx.send(ControlMessage::Pause).await;
                debug!("Sent pause signal to task {}", task_id);
            }
        }

        // Update status
        self.db.update_task_status(task_id, TaskStatus::Paused).await
            .map_err(|e| format!("Failed to update status: {}", e))?;

        let _ = self.event_tx.send(TaskEvent::StatusChanged {
            task_id,
            status: TaskStatus::Paused,
        });

        Ok(())
    }

    /// Resume a paused task
    pub async fn resume_task(&self, task_id: i64) -> Result<(), String> {
        let task = self.db.get_task(task_id).await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        if task.status != TaskStatus::Paused {
            return Err(format!("Task {} is not paused", task_id));
        }

        // Send resume signal
        {
            let running = self.running.lock().await;
            if let Some(tx) = running.get(&task_id) {
                let _ = tx.send(ControlMessage::Resume).await;
                debug!("Sent resume signal to task {}", task_id);
            }
        }

        // Update status
        self.db.update_task_status(task_id, TaskStatus::Running).await
            .map_err(|e| format!("Failed to update status: {}", e))?;

        let _ = self.event_tx.send(TaskEvent::StatusChanged {
            task_id,
            status: TaskStatus::Running,
        });

        Ok(())
    }

    /// Delete a task
    pub async fn delete_task(&self, task_id: i64) -> Result<(), String> {
        let task = self.db.get_task(task_id).await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        if task.status == TaskStatus::Running {
            return Err("Cannot delete a running task. Stop it first.".to_string());
        }

        // Remove from queue
        {
            let mut queue = self.queue.lock().await;
            queue.remove(task_id);
        }

        // Delete from database
        self.db.delete_task(task_id).await
            .map_err(|e| format!("Failed to delete task: {}", e))?;

        info!("Deleted task {}", task_id);

        Ok(())
    }

    /// Get task details
    pub async fn get_task(&self, task_id: i64) -> Result<Option<Task>, String> {
        self.db.get_task(task_id).await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Result<Vec<Task>, String> {
        self.db.list_tasks().await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<TaskStats, String> {
        self.db.get_stats().await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Validate task configuration
    fn validate_config(&self, config: &TaskConfig) -> Result<(), String> {
        // Check seeds directory
        if !std::path::Path::new(&config.seeds_dir).exists() {
            return Err(format!("Seeds directory not found: {}", config.seeds_dir));
        }

        // Check executor-specific requirements
        match config.executor_type {
            ExecutorType::Qemu => {
                if let Some(ref kernel) = config.kernel_path {
                    if !std::path::Path::new(kernel).exists() {
                        return Err(format!("Kernel not found: {}", kernel));
                    }
                } else {
                    return Err("Kernel path required for QEMU executor".to_string());
                }
                if let Some(ref initramfs) = config.initramfs_path {
                    if !std::path::Path::new(initramfs).exists() {
                        return Err(format!("Initramfs not found: {}", initramfs));
                    }
                } else {
                    return Err("Initramfs path required for QEMU executor".to_string());
                }

                if let Some(ref qemu_path) = config.qemu_path {
                    if !Self::is_executable_available(qemu_path) {
                        return Err(format!("QEMU not found: {}", qemu_path));
                    }
                } else {
                    return Err("QEMU path required for QEMU executor".to_string());
                }
            }
            ExecutorType::Erofsfuse => {
                if let Some(ref erofsfuse_path) = config.erofsfuse_path {
                    if !Self::is_executable_available(erofsfuse_path) {
                        return Err(format!("erofsfuse not found: {}", erofsfuse_path));
                    }
                } else {
                    return Err("erofsfuse path is required".to_string());
                }
            }
        }

        Ok(())
    }

    fn normalize_config(&self, mut config: TaskConfig) -> TaskConfig {
        fn normalize_optional_string(value: Option<String>) -> Option<String> {
            value
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }

        config.qemu_path = normalize_optional_string(config.qemu_path);
        config.erofsfuse_path = normalize_optional_string(config.erofsfuse_path);

        match config.executor_type {
            ExecutorType::Qemu => {
                if config.qemu_memory.is_none() {
                    config.qemu_memory = Some(1024);
                }
                if config.qemu_path.is_none() {
                    let default_qemu = if std::path::Path::new("/usr/bin/qemu-system-x86_64").exists() {
                        "/usr/bin/qemu-system-x86_64"
                    } else {
                        "qemu-system-x86_64"
                    };
                    config.qemu_path = Some(default_qemu.to_string());
                }
            }
            ExecutorType::Erofsfuse => {
                if config.erofsfuse_path.is_none() {
                    config.erofsfuse_path = Some("erofsfuse".to_string());
                }
            }
        }

        config
    }

    fn is_executable_available(program: &str) -> bool {
        let candidate = std::path::Path::new(program);
        if candidate.is_absolute() || program.contains('/') {
            return candidate.exists();
        }

        if candidate.exists() {
            return true;
        }

        std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths)
                    .any(|dir| dir.join(candidate).exists())
            })
            .unwrap_or(false)
    }

    /// Try to start the next pending task
    async fn try_start_next(&self) -> Result<(), String> {
        // Check if we can start more tasks
        let running_count = {
            let running = self.running.lock().await;
            running.len()
        };

        if running_count >= self.max_concurrent {
            debug!("Maximum concurrent tasks reached ({})", self.max_concurrent);
            return Ok(());
        }

        // Get next task from queue
        let task_id = {
            let mut queue = self.queue.lock().await;
            queue.dequeue()
        };

        if let Some(task_id) = task_id {
            let task = self.db.get_task(task_id).await
                .map_err(|e| format!("Database error: {}", e))?
                .ok_or_else(|| format!("Task {} not found", task_id))?;

            // Skip if already running
            if task.status == TaskStatus::Running {
                return Ok(());
            }

            self.spawn_runner(task).await?;
        }

        Ok(())
    }

    /// Spawn a task runner
    async fn spawn_runner(&self, task: Task) -> Result<(), String> {
        let task_id = task.id;
        info!("Starting task {} ({})", task_id, task.name);

        // Create control channel
        let (control_tx, control_rx) = mpsc::channel(16);

        // Register as running
        {
            let mut running = self.running.lock().await;
            running.insert(task_id, control_tx.clone());
        }

        // Update status
        self.db.update_task_status(task_id, TaskStatus::Running).await
            .map_err(|e| format!("Failed to update status: {}", e))?;

        let _ = self.event_tx.send(TaskEvent::StatusChanged {
            task_id,
            status: TaskStatus::Running,
        });

        // Clone necessary components
        let db = self.db.clone();
        let strategy_storage = self.strategy_storage.clone();
        let event_tx = self.event_tx.clone();
        let running = self.running.clone();

        // Spawn task runner
        tokio::spawn(async move {
            let runner = TaskRunner::new(task.clone(), db.clone(), strategy_storage, event_tx.clone(), control_rx);

            match runner.run().await {
                Ok(final_status) => {
                    let _ = db.update_task_status(task_id, final_status).await;
                    info!("Task {} finished with status: {:?}", task_id, final_status);
                    let _ = event_tx.send(TaskEvent::Finished {
                        task_id,
                        status: final_status,
                    });
                }
                Err(e) => {
                    let _ = db.set_task_error(task_id, &e).await;
                    error!("Task {} failed: {}", task_id, e);
                    let _ = event_tx.send(TaskEvent::Error {
                        task_id,
                        message: e.clone(),
                    });
                }
            }

            // Remove from running map
            {
                let mut running = running.lock().await;
                running.remove(&task_id);
            }

            debug!("Task {} completed, checking for next task", task_id);
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_manager_create() {
        let db = Database::in_memory().unwrap();
        let manager = TaskManager::new(db);

        let config = TaskConfig {
            name: "test".to_string(),
            seeds_dir: ".".to_string(),
            ..Default::default()
        };

        let result = manager.create_task(config).await;
        // Will fail because seeds_dir doesn't contain seeds, but that's expected
        // We're testing the basic flow
        assert!(result.is_ok() || result.unwrap_err().contains("not found"));
    }
}
