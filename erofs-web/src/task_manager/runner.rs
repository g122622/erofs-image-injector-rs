//! Task runner implementation

use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::collections::HashSet;

use tokio::process::Command;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

#[cfg(unix)]
use nix::sys::signal::{kill as send_signal, Signal};
#[cfg(unix)]
use nix::unistd::Pid;

use crate::db::Database;
use crate::strategy::StrategyStorage;
use crate::task_manager::ControlMessage;
use crate::task_manager::TaskEvent;
use crate::types::*;

/// Task runner that executes fuzzing
pub struct TaskRunner {
    /// Task to run
    task: Task,
    /// Database
    db: Database,
    /// Strategy storage
    strategy_storage: StrategyStorage,
    /// Event broadcaster
    event_tx: broadcast::Sender<TaskEvent>,
    /// Control channel
    control_rx: mpsc::Receiver<ControlMessage>,
    /// Pause state
    paused: bool,
    /// Known crash files (to avoid re-recording)
    known_crashes: HashSet<String>,
    /// Current mutator being used (parsed from fuzzer output)
    current_mutator: Option<String>,
    /// Current iteration (parsed from fuzzer output)
    current_iteration: u64,
    /// Current crash count (parsed from fuzzer output)
    current_crashes: u64,
    /// Current speed (parsed from fuzzer output)
    current_speed: f64,
    /// Total seeds count (parsed from fuzzer output)
    total_seeds: usize,
    /// Max iterations (parsed from fuzzer output)
    max_iterations: u64,
}

impl TaskRunner {
    /// Create a new task runner
    pub fn new(
        task: Task,
        db: Database,
        strategy_storage: StrategyStorage,
        event_tx: broadcast::Sender<TaskEvent>,
        control_rx: mpsc::Receiver<ControlMessage>,
    ) -> Self {
        Self {
            task,
            db,
            strategy_storage,
            event_tx,
            control_rx,
            paused: false,
            known_crashes: HashSet::new(),
            current_mutator: None,
            current_iteration: 0,
            current_crashes: 0,
            current_speed: 0.0,
            total_seeds: 0,
            max_iterations: 0,
        }
    }

    /// Run the task
    pub async fn run(mut self) -> Result<TaskStatus, String> {
        info!("Starting fuzzer for task {}", self.task.id);

        // Create output directory
        let output_dir = PathBuf::from(&self.task.output_dir);
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir)
                .map_err(|e| format!("Failed to create output directory: {}", e))?;
        }

        // Validate seeds exist
        let seeds = self.load_seeds(&PathBuf::from(&self.task.seeds_dir))?;
        if seeds == 0 {
            return Err("No seed files found".to_string());
        }

        info!("Loaded {} seeds for task {}", seeds, self.task.id);

        // Record existing crashes before starting
        self.scan_existing_crashes().await?;

        // Spawn real fuzzer subprocess (same executable, without --web)
        let mut child = self.spawn_fuzzer_process().await?;

        let mut last_update = Instant::now();

        // Spawn a task to read stdout and extract mutator info
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(64);
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let task_id = self.task.id;
        let stdout_task = tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                // Send line to channel for processing
                if stdout_tx.send(line).await.is_err() {
                    break;
                }
            }
            info!("[Task {}] Stdout reader finished", task_id);
        });

        loop {
            // Process stdout lines for progress and mutator info
            while let Ok(line) = stdout_rx.try_recv() {
                // Parse progress: [PROGRESS] iteration=N crashes=N speed=F
                if line.starts_with("[PROGRESS]") {
                    // Parse: [PROGRESS] iteration=123 crashes=5 speed=1.50
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if let Some(val) = part.strip_prefix("iteration=") {
                            self.current_iteration = val.parse().unwrap_or(self.current_iteration);
                        } else if let Some(val) = part.strip_prefix("crashes=") {
                            self.current_crashes = val.parse().unwrap_or(self.current_crashes);
                        } else if let Some(val) = part.strip_prefix("speed=") {
                            self.current_speed = val.parse().unwrap_or(self.current_speed);
                        }
                    }
                    debug!("[Task {}] Progress from fuzzer: iter={}, crashes={}, speed={}",
                           self.task.id, self.current_iteration, self.current_crashes, self.current_speed);
                }
                // Parse fuzzer start: [FUZZER] start seeds=N max_iterations=N
                else if line.starts_with("[FUZZER] start") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if let Some(val) = part.strip_prefix("seeds=") {
                            self.total_seeds = val.parse().unwrap_or(0);
                        } else if let Some(val) = part.strip_prefix("max_iterations=") {
                            self.max_iterations = val.parse().unwrap_or(0);
                        }
                    }
                    info!("[Task {}] Fuzzer started: seeds={}, max_iterations={}",
                          self.task.id, self.total_seeds, self.max_iterations);
                }
                // Parse mutator: [MUTATOR] xxx
                else if line.contains("[MUTATOR]") {
                    if let Some(mutator) = line.split("[MUTATOR]").nth(1) {
                        let mutator = mutator.trim().to_string();
                        debug!("[Task {}] Mutator: {}", self.task.id, mutator);
                        self.current_mutator = Some(mutator);
                    }
                }
            }

            while let Ok(msg) = self.control_rx.try_recv() {
                match msg {
                    ControlMessage::Stop => {
                        self.stop_child(&mut child).await;
                        let _ = stdout_task.await;
                        info!("Task {} stopped at iteration {}", self.task.id, self.current_iteration);
                        return Ok(TaskStatus::Cancelled);
                    }
                    ControlMessage::Pause => {
                        self.paused = true;
                        self.pause_child(&child);
                    }
                    ControlMessage::Resume => {
                        self.paused = false;
                        self.resume_child(&child);
                    }
                }
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    // Drain remaining stdout
                    while let Ok(line) = stdout_rx.try_recv() {
                        if line.starts_with("[PROGRESS]") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            for part in parts {
                                if let Some(val) = part.strip_prefix("iteration=") {
                                    self.current_iteration = val.parse().unwrap_or(self.current_iteration);
                                } else if let Some(val) = part.strip_prefix("crashes=") {
                                    self.current_crashes = val.parse().unwrap_or(self.current_crashes);
                                } else if let Some(val) = part.strip_prefix("speed=") {
                                    self.current_speed = val.parse().unwrap_or(self.current_speed);
                                }
                            }
                        } else if line.contains("[MUTATOR]") {
                            if let Some(mutator) = line.split("[MUTATOR]").nth(1) {
                                self.current_mutator = Some(mutator.trim().to_string());
                            }
                        }
                    }
                    let _ = stdout_task.await;

                    // Final progress update using actual fuzzer data
                    self.update_progress(self.current_iteration, self.current_crashes, self.current_speed).await?;

                    if status.success() {
                        info!(
                            "Task {} completed (exit code: {:?})",
                            self.task.id,
                            status.code()
                        );
                        return Ok(TaskStatus::Completed);
                    }

                    return Err(format!(
                        "Fuzzer process exited with status {:?}",
                        status.code()
                    ));
                }
                Ok(None) => {
                    // still running
                }
                Err(e) => {
                    let _ = stdout_task.await;
                    return Err(format!("Failed to poll fuzzer process: {}", e));
                }
            }

            // Update progress periodically using real data from fuzzer
            if last_update.elapsed() > Duration::from_secs(1) {
                // Use actual values from fuzzer output
                self.update_progress(self.current_iteration, self.current_crashes, self.current_speed).await?;
                last_update = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Load seed files from directory
    fn load_seeds(&self, seeds_dir: &PathBuf) -> Result<usize, String> {
        let mut seeds = 0usize;

        if !seeds_dir.exists() {
            return Err(format!("Seeds directory not found: {:?}", seeds_dir));
        }

        let entries = std::fs::read_dir(seeds_dir)
            .map_err(|e| format!("Failed to read seeds directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext == "erofs" || ext == "img" || ext == "" {
                    match std::fs::read(&path) {
                        Ok(_) => seeds += 1,
                        Err(e) => {
                            warn!("Failed to read seed {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(seeds)
    }

    /// Scan existing crash files before starting the task
    /// Records them to database if not already recorded, and tracks them to avoid duplicates
    async fn scan_existing_crashes(&mut self) -> Result<(), String> {
        let out = PathBuf::from(&self.task.output_dir);
        info!("[scan_existing_crashes] Output dir: {:?}, exists: {}", out, out.exists());
        if !out.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&out)
            .map_err(|e| format!("Failed to read output directory: {}", e))?;

        // Get existing crashes from database to avoid duplicates
        let existing_db_crashes = self.db.list_crashes(&crate::types::CrashFilter {
            task_id: Some(self.task.id),
            ..Default::default()
        }).await
            .map_err(|e| format!("Failed to list existing crashes: {}", e))?;

        info!("[scan_existing_crashes] Found {} existing crashes in database for task {}", existing_db_crashes.len(), self.task.id);

        let existing_image_paths: std::collections::HashSet<String> = existing_db_crashes
            .iter()
            .map(|c| c.image_path.clone())
            .collect();

        let mut new_crashes_recorded = 0;
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                info!("[scan_existing_crashes] Checking file: {}", name);
                if (name.starts_with("crash-") || name.starts_with("seed-crash-"))
                    && name.ends_with(".erofs")
                {
                    let path = entry.path();
                    let image_path = path.to_string_lossy().to_string();

                    // Track in known_crashes set
                    self.known_crashes.insert(name.to_string());

                    // Record to database if not already recorded
                    if !existing_image_paths.contains(&image_path) {
                        let crash_type = self.parse_crash_type_from_filename(name);
                        info!("[scan_existing_crashes] Recording new crash: {} (type: {:?})", name, crash_type);

                        match self.db.create_crash(
                            self.task.id,
                            0, // iteration 0 for pre-existing crashes
                            crash_type.clone(),
                            None,
                            &image_path,
                            None,
                        ).await {
                            Ok(crash_id) => {
                                info!(
                                    "[scan_existing_crashes] Recorded existing crash #{} ({}): {}",
                                    crash_id, crash_type, name
                                );
                                new_crashes_recorded += 1;

                                // Emit event
                                let _ = self.event_tx.send(TaskEvent::CrashFound {
                                    task_id: self.task.id,
                                    crash_id,
                                    crash_type,
                                    iteration: 0,
                                });

                                // Update task's total_crashes
                                self.task.total_crashes += 1;
                            }
                            Err(e) => {
                                warn!("[scan_existing_crashes] Failed to record existing crash {}: {}", name, e);
                            }
                        }
                    } else {
                        info!("[scan_existing_crashes] Crash already in database: {}", name);
                    }
                }
            }
        }

        info!("[scan_existing_crashes] Recorded {} new crashes, total known: {}", new_crashes_recorded, self.known_crashes.len());
        Ok(())
    }

    /// Scan for new crash files and record them in the database
    async fn scan_for_new_crashes(&mut self, iteration: u64) -> Result<usize, String> {
        let out = PathBuf::from(&self.task.output_dir);
        if !out.exists() {
            return Ok(0);
        }

        let entries = match std::fs::read_dir(&out) {
            Ok(e) => e,
            Err(_) => return Ok(0),
        };

        let mut new_crash_count = 0;

        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if (name.starts_with("crash-") || name.starts_with("seed-crash-"))
                    && name.ends_with(".erofs")
                    && !self.known_crashes.contains(name)
                {
                    // New crash found
                    let path = entry.path();
                    let image_path = path.to_string_lossy().to_string();

                    // Parse crash type from filename
                    let crash_type = self.parse_crash_type_from_filename(name);

                    // Create crash record in database
                    match self.db.create_crash(
                        self.task.id,
                        iteration,
                        crash_type,
                        None, // signal
                        &image_path,
                        None, // log_path
                    ).await {
                        Ok(crash_id) => {
                            info!(
                                "Recorded new crash #{} ({}): {}",
                                crash_id, crash_type, name
                            );

                            // Emit event
                            let _ = self.event_tx.send(TaskEvent::CrashFound {
                                task_id: self.task.id,
                                crash_id,
                                crash_type,
                                iteration,
                            });

                            new_crash_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to record crash {}: {}", name, e);
                        }
                    }

                    // Mark as known
                    self.known_crashes.insert(name.to_string());
                }
            }
        }

        Ok(new_crash_count)
    }

    /// Parse crash type from filename
    fn parse_crash_type_from_filename(&self, filename: &str) -> CrashType {
        // Filenames are like: crash-<hash>-<type>.erofs
        // e.g., crash-18a141b4a6fec0b4-kernel-oops.erofs
        // e.g., crash-<hash>-signal-<num>.erofs
        // e.g., crash-<hash>-asan.erofs

        let lower = filename.to_lowercase();

        if lower.contains("kernel-panic") || lower.contains("kernel_panic") {
            CrashType::KernelPanic
        } else if lower.contains("kernel-oops") || lower.contains("kernel_oops") {
            CrashType::KernelOops
        } else if lower.contains("asan") {
            CrashType::Asan
        } else {
            // Default to Signal for other crash types
            CrashType::Signal
        }
    }

    async fn spawn_fuzzer_process(&self) -> Result<tokio::process::Child, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to locate current executable: {}", e))?;

        let mut cmd = Command::new(exe);
        cmd.arg("--seeds")
            .arg(&self.task.seeds_dir)
            .arg("--output")
            .arg(&self.task.output_dir)
            .arg("--timeout")
            .arg(self.task.timeout_seconds.to_string())
            .arg("--iterations")
            .arg(self.task.max_iterations.to_string())
            .arg("--workers")
            .arg(self.task.workers.to_string())
            .arg("--log-level")
            .arg("info");

        match self.task.executor_type {
            ExecutorType::Qemu => {
                cmd.arg("--executor").arg("qemu");
                if let Some(kernel) = &self.task.kernel_path {
                    cmd.arg("--kernel").arg(kernel);
                }
                if let Some(initramfs) = &self.task.initramfs_path {
                    cmd.arg("--initramfs").arg(initramfs);
                }
                if let Some(qemu_path) = &self.task.qemu_path {
                    cmd.arg("--qemu-path").arg(qemu_path);
                }
                if let Some(mem) = self.task.qemu_memory {
                    cmd.arg("--qemu-memory").arg(mem.to_string());
                }
            }
            ExecutorType::Erofsfuse => {
                if let Some(erofsfuse_path) = &self.task.erofsfuse_path {
                    cmd.arg("--erofsfuse-path").arg(erofsfuse_path);
                }
            }
        }

        // Load strategy configuration and pass to fuzzer
        if let Some(strategy_id) = self.task.strategy_id {
            if let Some(strategy) = self.strategy_storage.get(strategy_id).await {
                // Build strategy config JSON for fuzzer
                let mut mutators = std::collections::HashMap::new();
                for (mutator_type, config) in &strategy.mutators {
                    if config.enabled {
                        mutators.insert(
                            mutator_type.to_string(),
                            serde_json::json!({
                                "enabled": true,
                                "weight": config.weight
                            })
                        );
                    }
                }

                let strategy_json = serde_json::to_string(&serde_json::json!({
                    "mutators": mutators
                })).map_err(|e| format!("Failed to serialize strategy: {}", e))?;

                cmd.arg("--strategy-config").arg(&strategy_json);
                info!("[Task {}] Using strategy '{}' (id={}): {} mutators enabled",
                      self.task.id, strategy.name, strategy_id, mutators.len());
            } else {
                warn!("[Task {}] Strategy {} not found, using defaults", self.task.id, strategy_id);
            }
        }

        debug!("Spawning fuzz subprocess for task {}", self.task.id);
        // Capture stdout to parse mutator info
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        info!("[Task {}] Fuzzer command: {:?}", self.task.id, cmd);
        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn fuzz subprocess: {}", e))?;
        info!("[Task {}] Fuzzer process spawned with pid {:?}", self.task.id, child.id());
        Ok(child)
    }

    async fn stop_child(&self, child: &mut tokio::process::Child) {
        let _ = child.kill().await;
    }

    #[cfg(unix)]
    fn pause_child(&self, child: &tokio::process::Child) {
        if let Some(pid) = child.id() {
            let _ = send_signal(Pid::from_raw(pid as i32), Signal::SIGSTOP);
        }
    }

    #[cfg(not(unix))]
    fn pause_child(&self, _child: &tokio::process::Child) {}

    #[cfg(unix)]
    fn resume_child(&self, child: &tokio::process::Child) {
        if let Some(pid) = child.id() {
            let _ = send_signal(Pid::from_raw(pid as i32), Signal::SIGCONT);
        }
    }

    #[cfg(not(unix))]
    fn resume_child(&self, _child: &tokio::process::Child) {}

    /// Update task progress
    async fn update_progress(&self, iteration: u64, crashes: u64, speed: f64) -> Result<(), String> {
        self.db.update_task_progress(self.task.id, iteration, crashes, speed).await
            .map_err(|e| format!("Failed to update progress: {}", e))?;

        info!("[Task {}] Progress: iter={}, crashes={}, speed={}, mutator={:?}",
              self.task.id, iteration, crashes, speed, self.current_mutator);

        let _ = self.event_tx.send(TaskEvent::Progress {
            task_id: self.task.id,
            iteration,
            crashes,
            speed,
            current_mutator: self.current_mutator.clone(),
        });

        Ok(())
    }
}
