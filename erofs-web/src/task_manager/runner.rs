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
use crate::task_manager::ControlMessage;
use crate::task_manager::TaskEvent;
use crate::types::*;

/// Task runner that executes fuzzing
pub struct TaskRunner {
    /// Task to run
    task: Task,
    /// Database
    db: Database,
    /// Event broadcaster
    event_tx: broadcast::Sender<TaskEvent>,
    /// Control channel
    control_rx: mpsc::Receiver<ControlMessage>,
    /// Pause state
    paused: bool,
    /// Known crash files (to avoid re-recording)
    known_crashes: HashSet<String>,
}

impl TaskRunner {
    /// Create a new task runner
    pub fn new(
        task: Task,
        db: Database,
        event_tx: broadcast::Sender<TaskEvent>,
        control_rx: mpsc::Receiver<ControlMessage>,
    ) -> Self {
        Self {
            task,
            db,
            event_tx,
            control_rx,
            paused: false,
            known_crashes: HashSet::new(),
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

        // Track progress/heartbeat
        let mut iteration: u64 = self.task.current_iteration;
        let start_time = Instant::now();
        let mut last_update = Instant::now();
        let mut last_progress_iteration = iteration;
        let mut last_progress_time = start_time;
        let mut total_crashes: u64 = self.task.total_crashes;

        loop {
            while let Ok(msg) = self.control_rx.try_recv() {
                match msg {
                    ControlMessage::Stop => {
                        self.stop_child(&mut child).await;
                        info!("Task {} stopped at iteration {}", self.task.id, iteration);
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
                    // Scan for new crashes one final time
                    let new_crashes = self.scan_for_new_crashes(iteration).await?;
                    total_crashes += new_crashes as u64;

                    // Final progress flush before returning
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        iteration as f64 / elapsed
                    } else {
                        0.0
                    };
                    self.update_progress(iteration, total_crashes, speed).await?;

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
                    return Err(format!("Failed to poll fuzzer process: {}", e));
                }
            }

            // Heartbeat/progress update periodically.
            // We cannot read exact iteration from subprocess yet, so approximate by elapsed time.
            // This avoids the old misleading instant completion behavior while keeping UI responsive.
            if last_update.elapsed() > Duration::from_secs(1) {
                // Conservative synthetic progress estimate for UI only.
                // Uses +1 tick/sec and real crash count from output directory.
                if !self.paused {
                    iteration = iteration.saturating_add(1);
                }

                // Scan for new crashes
                let new_crashes = self.scan_for_new_crashes(iteration).await?;
                total_crashes += new_crashes as u64;

                let now = Instant::now();
                let dt = now.duration_since(last_progress_time).as_secs_f64();
                let speed = if dt > 0.0 {
                    (iteration.saturating_sub(last_progress_iteration)) as f64 / dt
                } else {
                    0.0
                };

                self.update_progress(iteration, total_crashes, speed).await?;
                last_update = Instant::now();
                last_progress_iteration = iteration;
                last_progress_time = now;
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

        debug!("Spawning fuzz subprocess for task {}", self.task.id);
        cmd.spawn()
            .map_err(|e| format!("Failed to spawn fuzz subprocess: {}", e))
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

        let _ = self.event_tx.send(TaskEvent::Progress {
            task_id: self.task.id,
            iteration,
            crashes,
            speed,
        });

        Ok(())
    }
}
