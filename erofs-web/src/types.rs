//! Shared types for the web console

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Waiting to be started
    Pending,
    /// Currently running
    Running,
    /// Paused by user
    Paused,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Paused => write!(f, "paused"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

/// Executor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorType {
    /// User-space erofsfuse testing
    Erofsfuse,
    /// QEMU kernel testing
    Qemu,
}

impl Default for ExecutorType {
    fn default() -> Self {
        Self::Erofsfuse
    }
}

impl std::fmt::Display for ExecutorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorType::Erofsfuse => write!(f, "erofsfuse"),
            ExecutorType::Qemu => write!(f, "qemu"),
        }
    }
}

impl std::str::FromStr for ExecutorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "erofsfuse" => Ok(Self::Erofsfuse),
            "qemu" => Ok(Self::Qemu),
            _ => Err(format!("Invalid executor type: {}", s)),
        }
    }
}

/// Crash type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CrashType {
    /// Signal crash (SIGSEGV, etc.)
    Signal,
    /// AddressSanitizer error
    Asan,
    /// Kernel panic
    KernelPanic,
    /// Kernel oops
    KernelOops,
}

impl std::fmt::Display for CrashType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrashType::Signal => write!(f, "Signal"),
            CrashType::Asan => write!(f, "ASan"),
            CrashType::KernelPanic => write!(f, "KernelPanic"),
            CrashType::KernelOops => write!(f, "KernelOops"),
        }
    }
}

impl std::str::FromStr for CrashType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Signal" => Ok(Self::Signal),
            "ASan" | "Asan" => Ok(Self::Asan),
            "KernelPanic" => Ok(Self::KernelPanic),
            "KernelOops" => Ok(Self::KernelOops),
            _ => Err(format!("Invalid crash type: {}", s)),
        }
    }
}

/// Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task name
    pub name: String,
    /// Executor type
    pub executor_type: ExecutorType,
    /// Seeds directory
    pub seeds_dir: String,
    /// Output directory for crashes
    pub output_dir: String,
    /// Timeout per execution in seconds
    pub timeout_seconds: u64,
    /// Maximum iterations (0 = unlimited)
    pub max_iterations: u64,
    /// Number of parallel workers
    #[serde(default = "default_workers")]
    pub workers: usize,
    /// QEMU memory in MB (for QEMU executor)
    pub qemu_memory: Option<usize>,
    /// Kernel path (for QEMU executor)
    pub kernel_path: Option<String>,
    /// Initramfs path (for QEMU executor)
    pub initramfs_path: Option<String>,
    /// QEMU binary path
    pub qemu_path: Option<String>,
    /// Erofsfuse binary path
    pub erofsfuse_path: Option<String>,
    /// Strategy template ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_id: Option<i64>,
}

fn default_workers() -> usize {
    1
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            executor_type: ExecutorType::Erofsfuse,
            seeds_dir: "./seeds".to_string(),
            output_dir: "./crashes".to_string(),
            timeout_seconds: 60,
            max_iterations: 0,
            workers: 1,
            qemu_memory: None,
            kernel_path: None,
            initramfs_path: None,
            qemu_path: None,
            erofsfuse_path: None,
            strategy_id: None,
        }
    }
}

/// Task model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID
    pub id: i64,
    /// Task name
    pub name: String,
    /// Current status
    pub status: TaskStatus,
    /// Executor type
    pub executor_type: ExecutorType,
    /// Seeds directory
    pub seeds_dir: String,
    /// Output directory
    pub output_dir: String,
    /// Timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum iterations
    pub max_iterations: u64,
    /// Number of workers
    pub workers: usize,
    /// QEMU memory (if applicable)
    pub qemu_memory: Option<usize>,
    /// Kernel path (if applicable)
    pub kernel_path: Option<String>,
    /// Initramfs path (if applicable)
    pub initramfs_path: Option<String>,
    /// QEMU binary path (if applicable)
    pub qemu_path: Option<String>,
    /// Erofsfuse binary path (if applicable)
    pub erofsfuse_path: Option<String>,
    /// Strategy template ID
    pub strategy_id: Option<i64>,
    /// Current iteration count
    pub current_iteration: u64,
    /// Total crashes found
    pub total_crashes: u64,
    /// Executions per second
    pub exec_per_sec: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Start timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Finish timestamp
    pub finished_at: Option<DateTime<Utc>>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Create task request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// Task configuration
    #[serde(flatten)]
    pub config: TaskConfig,
}

/// Update task request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTaskRequest {
    /// Task name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Maximum iterations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u64>,
}

/// Crash model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crash {
    /// Crash ID
    pub id: i64,
    /// Associated task ID
    pub task_id: i64,
    /// Iteration number when crash occurred
    pub iteration: u64,
    /// Crash type
    pub crash_type: CrashType,
    /// Signal number (for Signal type)
    pub signal: Option<i32>,
    /// Path to crash image file
    pub image_path: String,
    /// Path to crash log file
    pub log_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// WebSocket message from client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to task updates
    Subscribe { task_id: i64 },
    /// Unsubscribe from task updates
    Unsubscribe { task_id: i64 },
    /// Subscribe to all task updates
    SubscribeAll,
}

/// WebSocket message to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Task progress update
    Progress {
        task_id: i64,
        iteration: u64,
        crashes: u64,
        speed: f64,
    },
    /// New crash found
    Crash {
        task_id: i64,
        crash_id: i64,
        crash_type: CrashType,
        iteration: u64,
    },
    /// Task status changed
    Status {
        task_id: i64,
        status: TaskStatus,
    },
    /// Task created
    TaskCreated {
        task: Task,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Strategy statistics update
    StrategyStats {
        task_id: i64,
        stats: StrategyStatsMessage,
    },
}

/// Strategy statistics message for WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStatsMessage {
    /// Strategy template ID
    pub strategy_id: Option<i64>,
    /// Strategy name
    pub strategy_name: String,
    /// Per-mutator statistics
    pub mutators: Vec<MutatorStatsMessage>,
    /// Total iterations
    pub total_iterations: u64,
    /// Total crashes
    pub total_crashes: u64,
    /// Whether adaptive weights are active
    pub adaptive_active: bool,
}

/// Mutator statistics message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutatorStatsMessage {
    /// Mutator type
    pub mutator: String,
    /// Total executions
    pub executions: u64,
    /// Crashes found
    pub crashes: u64,
    /// Current weight percentage
    pub weight_percent: f64,
    /// Crash rate
    pub crash_rate: f64,
}

/// Task statistics for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    /// Total tasks
    pub total: u64,
    /// Running tasks
    pub running: u64,
    /// Pending tasks
    pub pending: u64,
    /// Completed tasks
    pub completed: u64,
    /// Failed tasks
    pub failed: u64,
    /// Total crashes across all tasks
    pub total_crashes: u64,
    /// Total iterations across all tasks
    pub total_iterations: u64,
}

/// Crash filter for querying
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrashFilter {
    /// Filter by task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,
    /// Filter by crash type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash_type: Option<CrashType>,
    /// Limit results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

/// Reproduction script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductionScript {
    /// Script content
    pub script: String,
    /// Script type (bash, etc.)
    pub script_type: String,
    /// Description
    pub description: String,
}
