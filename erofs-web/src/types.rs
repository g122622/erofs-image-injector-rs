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
    /// Seeds directory (optional - derived from seed_ids if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeds_dir: Option<String>,
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
    /// Selected seed IDs from seed management
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_ids: Option<Vec<i64>>,
}

fn default_workers() -> usize {
    1
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            executor_type: ExecutorType::Erofsfuse,
            seeds_dir: None,
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
            seed_ids: None,
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
    /// Kernel version (detected at runtime)
    pub kernel_version: Option<String>,
    /// EROFS version (detected at runtime)
    pub erofs_version: Option<String>,
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
        /// Current mutator being used
        current_mutator: Option<String>,
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
    /// Real-time log message
    Log {
        task_id: i64,
        level: LogLevel,
        message: String,
        timestamp: i64,
    },
    /// Seed tracking during task execution
    SeedInfo {
        task_id: i64,
        /// Current seed name
        current_seed: Option<String>,
        /// Current seed index (0-based)
        seed_index: usize,
        /// Total seeds in the task
        total_seeds: usize,
    },
    /// Seed generation progress
    SeedGenerated {
        job_id: i64,
        seed_name: String,
        seed_id: i64,
        progress: f32,
    },
}

/// Log level for real-time logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
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

// ============================================================================
// Seed Management Types
// ============================================================================

/// Seed model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    /// Seed ID
    pub id: i64,
    /// Display name
    pub name: String,
    /// Path to the .erofs file
    pub file_path: String,
    /// File size in bytes
    pub file_size: i64,
    /// SHA256 checksum
    pub checksum: Option<String>,
    /// Generation configuration (JSON)
    pub config: SeedConfig,
    /// How many tasks have used this seed
    pub times_used: i64,
    /// Total crashes found using this seed
    pub crashes_found: i64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: Option<DateTime<Utc>>,
    /// Whether the seed file still exists
    pub is_valid: bool,
    /// Comma-separated tags
    pub tags: Option<String>,
}

/// Seed generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedConfig {
    /// Block size in bytes (512, 1024, 2048, 4096)
    pub block_size: u32,
    /// Volume name (max 16 bytes)
    pub volume_name: String,
    /// Compression configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<CompressionConfig>,
    /// Directory tree structure
    pub root: DirectoryTreeNode,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            block_size: 4096,
            volume_name: "erofs".to_string(),
            compression: None,
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(vec![]),
                xattr: None,
                mode: Some(0o40755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: None,
            tags: None,
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Compression level (algorithm-specific)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// Dictionary size (for LZMA/ZSTD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dict_size: Option<u32>,
}

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgorithm {
    Lz4,
    Lz4hc,
    Lzma,
    Zstd,
}

impl std::fmt::Display for CompressionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionAlgorithm::Lz4 => write!(f, "lz4"),
            CompressionAlgorithm::Lz4hc => write!(f, "lz4hc"),
            CompressionAlgorithm::Lzma => write!(f, "lzma"),
            CompressionAlgorithm::Zstd => write!(f, "zstd"),
        }
    }
}

impl std::str::FromStr for CompressionAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lz4" => Ok(Self::Lz4),
            "lz4hc" => Ok(Self::Lz4hc),
            "lzma" => Ok(Self::Lzma),
            "zstd" => Ok(Self::Zstd),
            _ => Err(format!("Invalid compression algorithm: {}", s)),
        }
    }
}

/// Directory tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryTreeNode {
    /// Name of the file or directory
    pub name: String,
    /// Type of node
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// File content configuration (for files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<FileContentConfig>,
    /// Child nodes (for directories)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DirectoryTreeNode>>,
    /// Extended attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xattr: Option<Vec<ExtendedAttribute>>,
    /// Unix permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u16>,
    /// Owner user ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    /// Owner group ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gid: Option<u32>,
    /// Symlink target (for symlinks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    File,
    Directory,
    Symlink,
}

/// File content configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContentConfig {
    /// Content type
    #[serde(rename = "type")]
    pub content_type: FileContentType,
    /// Text content (for text type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_content: Option<String>,
    /// Binary content as base64 (for binary type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_content: Option<String>,
    /// AFL generation config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afl_config: Option<AflContentConfig>,
    /// Random content config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_config: Option<RandomContentConfig>,
    /// Pattern content config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_config: Option<PatternContentConfig>,
}

impl Default for FileContentConfig {
    fn default() -> Self {
        Self {
            content_type: FileContentType::Text,
            text_content: None,
            binary_content: None,
            afl_config: None,
            random_config: None,
            pattern_config: None,
        }
    }
}

/// File content type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileContentType {
    Text,
    Binary,
    AflGenerated,
    Random,
    Pattern,
}

/// AFL content generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AflContentConfig {
    /// Size range [min, max] in bytes
    pub size_range: (usize, usize),
    /// Add AFL header
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_header: Option<bool>,
    /// AFL format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AflFormat>,
    /// Pattern to inject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_injection: Option<String>,
}

/// AFL format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AflFormat {
    Raw,
    Afl,
}

/// Random content configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomContentConfig {
    /// Size range [min, max] in bytes
    pub size_range: (usize, usize),
    /// Entropy level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy: Option<EntropyLevel>,
}

/// Entropy level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntropyLevel {
    Low,
    Medium,
    High,
}

/// Pattern content configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternContentConfig {
    /// Pattern string
    pub pattern: String,
    /// Repeat count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<usize>,
    /// Exact size (overrides repeat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

/// Extended attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedAttribute {
    /// Attribute name
    pub name: String,
    /// Attribute value (base64 for binary)
    pub value: String,
}

/// Create seed request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSeedRequest {
    /// Seed name
    pub name: String,
    /// Generation configuration
    pub config: SeedConfig,
    /// Number of seeds to generate (for batch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// Seed generation job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedGenerationJob {
    /// Job ID
    pub id: i64,
    /// Job status
    pub status: JobStatus,
    /// Progress percentage (0-100)
    pub progress: f32,
    /// Seeds generated so far
    pub seeds_generated: usize,
    /// Total seeds to generate
    pub seeds_total: usize,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Seed filter for querying
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeedFilter {
    /// Filter by validity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_valid: Option<bool>,
    /// Filter by tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Limit results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

/// Seed template for default configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Description
    pub description: String,
    /// Template configuration
    pub config: SeedConfig,
}

/// Seed statistics for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedTaskStats {
    /// Seed ID
    pub seed_id: i64,
    /// Seed name
    pub seed_name: String,
    /// Index in task's seed list
    pub seed_index: i64,
    /// Iterations run on this seed
    pub iterations: i64,
    /// Crashes found with this seed
    pub crashes: i64,
}
