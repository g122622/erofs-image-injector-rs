//! Task runner implementation

use std::path::PathBuf;
use std::time::{Duration, Instant};

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

        // Spawn real fuzzer subprocess (same executable, without --web)
        let mut child = self.spawn_fuzzer_process().await?;

        // Track progress/heartbeat
        let mut iteration: u64 = self.task.current_iteration;
        let start_time = Instant::now();
        let mut last_update = Instant::now();
        let mut last_progress_iteration = iteration;
        let mut last_progress_time = start_time;

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
                    // Final progress flush before returning
                    let crashes = self.count_crashes();
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        iteration as f64 / elapsed
                    } else {
                        0.0
                    };
                    self.update_progress(iteration, crashes, speed).await?;

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

                let now = Instant::now();
                let dt = now.duration_since(last_progress_time).as_secs_f64();
                let speed = if dt > 0.0 {
                    (iteration.saturating_sub(last_progress_iteration)) as f64 / dt
                } else {
                    0.0
                };
                let crashes = self.count_crashes();

                self.update_progress(iteration, crashes, speed).await?;
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

    fn count_crashes(&self) -> u64 {
        let out = PathBuf::from(&self.task.output_dir);
        let mut count = 0u64;
        if let Ok(entries) = std::fs::read_dir(out) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if (name.starts_with("crash-") || name.starts_with("seed-crash-"))
                        && name.ends_with(".erofs")
                    {
                        count = count.saturating_add(1);
                    }
                }
            }
        }
        count
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

/// Simple random number helper (placeholder for proper RNG)
fn rand_below(max: usize) -> usize {
    if max == 0 {
        return 0;
    }
    // Use a simple hash of current time for now
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (now as usize) % max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand_below() {
        let result = rand_below(10);
        assert!(result < 10);
    }
}
