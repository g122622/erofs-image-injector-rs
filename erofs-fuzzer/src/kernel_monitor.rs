//! Kernel Crash Monitor
//!
//! This module provides functionality to detect kernel panics, oops, and EROFS-related
//! errors from kernel log output.

use std::fmt;
use crate::executor_trait::ExecutionResult;

/// Types of kernel issues that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelIssue {
    /// Kernel panic
    Panic,
    /// Kernel oops
    Oops,
    /// General protection fault
    GeneralProtectionFault,
    /// Segmentation fault in kernel
    KernelSegfault,
    /// NULL pointer dereference
    NullPointerDereference,
    /// Memory corruption
    MemoryCorruption,
    /// EROFS-specific error
    ErofsError,
    /// Unknown error
    Unknown,
}

impl fmt::Display for KernelIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelIssue::Panic => write!(f, "Kernel Panic"),
            KernelIssue::Oops => write!(f, "Kernel Oops"),
            KernelIssue::GeneralProtectionFault => write!(f, "General Protection Fault"),
            KernelIssue::KernelSegfault => write!(f, "Kernel Segmentation Fault"),
            KernelIssue::NullPointerDereference => write!(f, "NULL Pointer Dereference"),
            KernelIssue::MemoryCorruption => write!(f, "Memory Corruption"),
            KernelIssue::ErofsError => write!(f, "EROFS Error"),
            KernelIssue::Unknown => write!(f, "Unknown Kernel Issue"),
        }
    }
}

/// Kernel monitor for detecting crashes and errors
#[derive(Debug, Clone)]
pub struct KernelMonitor {
    /// Known kernel panic patterns
    panic_patterns: Vec<&'static str>,
    /// Known kernel oops patterns
    oops_patterns: Vec<&'static str>,
    /// EROFS-specific error patterns
    erofs_patterns: Vec<&'static str>,
    /// Memory error patterns
    memory_patterns: Vec<&'static str>,
}

impl Default for KernelMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelMonitor {
    /// Create a new kernel monitor with default patterns
    pub fn new() -> Self {
        Self {
            panic_patterns: vec![
                "Kernel panic",
                "kernel panic",
                "KERNEL PANIC",
                "PANIC:",
            ],
            oops_patterns: vec![
                "Oops:",
                "OOPS:",
                "BUG:",
                "kernel BUG at",
                "invalid opcode:",
                "stack segment:",
                "general protection fault:",
                "GP fault",
            ],
            erofs_patterns: vec![
                "erofs:",
                "EROFS:",
                "erofs_readpage",
                "erofs_readpages",
                "erofs_lookup",
                "erofs_readdir",
                "erofs_fill_super",
                "erofs_get_block",
                "erofs_decode_inode",
                "EROFS error",
                "EROFS warning",
            ],
            memory_patterns: vec![
                "NULL pointer dereference",
                "null pointer dereference",
                "Unable to handle kernel NULL pointer",
                "Unable to handle kernel paging request",
                "kernel tried to execute NX-protected page",
                "corrupted stack",
                "Stack overflow",
                "kernel stack overflow",
                "BUG: bad page state",
                "BUG: unable to handle kernel",
                "divide error:",
                "invalid opcode:",
            ],
        }
    }

    /// Check a single line of kernel output for issues
    pub fn check_line(line: &str) -> Option<ExecutionResult> {
        // Use a temporary monitor instance
        let monitor = Self::new();
        monitor.detect_issue(line)
    }

    /// Detect kernel issues from a log line
    pub fn detect_issue(&self, line: &str) -> Option<ExecutionResult> {
        let line_lower = line.to_lowercase();

        // Check for kernel panic
        for pattern in &self.panic_patterns {
            if line.contains(pattern) || line_lower.contains(&pattern.to_lowercase()) {
                return Some(ExecutionResult::KernelPanic);
            }
        }

        // Check for kernel oops
        for pattern in &self.oops_patterns {
            if line.contains(pattern) || line_lower.contains(&pattern.to_lowercase()) {
                return Some(ExecutionResult::KernelOops);
            }
        }

        // Check for memory errors (often indicate oops/panic)
        for pattern in &self.memory_patterns {
            if line.contains(pattern) || line_lower.contains(&pattern.to_lowercase()) {
                return Some(ExecutionResult::KernelPanic);
            }
        }

        None
    }

    /// Analyze a complete kernel log
    pub fn analyze_log(&self, log: &str) -> Option<KernelCrashInfo> {
        let lines: Vec<&str> = log.lines().collect();
        let mut crash_info: Option<KernelCrashInfo> = None;

        for (idx, line) in lines.iter().enumerate() {
            // Check for panic
            if self.is_panic_line(line) {
                crash_info = Some(KernelCrashInfo {
                    issue_type: KernelIssue::Panic,
                    line_number: idx,
                    line: line.to_string(),
                    context: self.extract_context(&lines, idx),
                    stack_trace: self.extract_stack_trace(&lines, idx),
                });
            }

            // Check for oops (lower priority than panic)
            if crash_info.is_none() && self.is_oops_line(line) {
                crash_info = Some(KernelCrashInfo {
                    issue_type: KernelIssue::Oops,
                    line_number: idx,
                    line: line.to_string(),
                    context: self.extract_context(&lines, idx),
                    stack_trace: self.extract_stack_trace(&lines, idx),
                });
            }
        }

        crash_info
    }

    /// Check if line indicates a panic
    fn is_panic_line(&self, line: &str) -> bool {
        let line_lower = line.to_lowercase();
        for pattern in &self.panic_patterns {
            if line.contains(pattern) || line_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Check if line indicates an oops
    fn is_oops_line(&self, line: &str) -> bool {
        let line_lower = line.to_lowercase();
        for pattern in &self.oops_patterns {
            if line.contains(pattern) || line_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Check if line contains EROFS-specific output
    pub fn is_erofs_line(&self, line: &str) -> bool {
        let line_lower = line.to_lowercase();
        for pattern in &self.erofs_patterns {
            if line.contains(pattern) || line_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Extract context around a line (3 lines before and after)
    fn extract_context(&self, lines: &[&str], idx: usize) -> String {
        let start = idx.saturating_sub(3);
        let end = (idx + 4).min(lines.len());
        lines[start..end].join("\n")
    }

    /// Extract kernel stack trace starting from a line
    fn extract_stack_trace(&self, lines: &[&str], start_idx: usize) -> String {
        let mut trace = String::new();
        let mut in_trace = false;

        for line in lines.iter().skip(start_idx) {
            let trimmed = line.trim();
            let trimmed_lower = trimmed.to_lowercase();

            // Stack trace indicators
            if trimmed.contains("Call Trace:")
                || trimmed.contains("Stack:")
                || trimmed.contains("Backtrace:")
                || trimmed_lower.contains("stack trace:")
            {
                in_trace = true;
                trace.push_str(trimmed);
                trace.push('\n');
                continue;
            }

            // Stack trace lines typically start with addresses or function names
            if in_trace {
                // Common line forms:
                // [  1.234]  show_stack+0x50/0x70
                // ? function+0x10/0x20
                // RIP: 0010:function+0x12/0x34
                let looks_like_trace = trimmed.starts_with(' ')
                    || trimmed.starts_with('\t')
                    || trimmed.starts_with("0x")
                    || trimmed.starts_with('?')
                    || trimmed.contains("+0x")
                    || trimmed.contains("+0X")
                    || (trimmed.starts_with('[') && (trimmed.contains("+0x") || trimmed.contains("RIP:")));

                if looks_like_trace {
                    trace.push_str(trimmed);
                    trace.push('\n');
                } else if trimmed.is_empty() {
                    // Empty line might end the trace
                    break;
                } else if !trimmed.contains("+") && !trimmed.contains(":") {
                    // Probably not part of the trace anymore
                    break;
                }
            }
        }

        trace
    }

    /// Check if a log contains EROFS-related errors
    pub fn find_erofs_errors(&self, log: &str) -> Vec<String> {
        log.lines()
            .filter(|line| self.is_erofs_line(line))
            .map(|s| s.to_string())
            .collect()
    }
}

/// Information about a kernel crash
#[derive(Debug, Clone)]
pub struct KernelCrashInfo {
    /// Type of issue
    pub issue_type: KernelIssue,
    /// Line number where the issue was detected
    pub line_number: usize,
    /// The line that triggered detection
    pub line: String,
    /// Context around the issue
    pub context: String,
    /// Stack trace if available
    pub stack_trace: String,
}

impl fmt::Display for KernelCrashInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Kernel Issue: {}", self.issue_type)?;
        writeln!(f, "Line {}: {}", self.line_number, self.line)?;
        if !self.stack_trace.is_empty() {
            writeln!(f, "\nStack Trace:")?;
            writeln!(f, "{}", self.stack_trace)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_monitor_creation() {
        let monitor = KernelMonitor::new();
        assert!(!monitor.panic_patterns.is_empty());
        assert!(!monitor.oops_patterns.is_empty());
        assert!(!monitor.erofs_patterns.is_empty());
    }

    #[test]
    fn test_panic_detection() {
        let monitor = KernelMonitor::new();

        assert!(monitor.detect_issue("Kernel panic - not syncing: VFS: Unable to mount root fs").is_some());
        assert!(monitor.detect_issue("Kernel Panic - forced reset").is_some());
        assert!(monitor.detect_issue("PANIC: test failure").is_some());
    }

    #[test]
    fn test_oops_detection() {
        let monitor = KernelMonitor::new();

        assert!(monitor.detect_issue("Oops: 0000 [#1] SMP").is_some());
        assert!(monitor.detect_issue("BUG: unable to handle kernel NULL pointer dereference").is_some());
        assert!(monitor.detect_issue("general protection fault: 0000 [#1]").is_some());
    }

    #[test]
    fn test_erofs_detection() {
        let monitor = KernelMonitor::new();

        assert!(monitor.is_erofs_line("erofs: mounted filesystem"));
        assert!(monitor.is_erofs_line("EROFS error: invalid superblock"));
        assert!(monitor.is_erofs_line("erofs_lookup: failed"));
        assert!(!monitor.is_erofs_line("ext4: mounted filesystem"));
    }

    #[test]
    fn test_analyze_log() {
        let monitor = KernelMonitor::new();

        let log = r#"[    0.000000] Linux version 6.8.0
[    1.234567] erofs: mounted filesystem
[    2.345678] Kernel panic - not syncing: Attempted to kill init!
[    2.345679] Call Trace:
[    2.345680]  show_stack+0x50/0x70
[    2.345681]  dump_stack_lvl+0x4a/0x60
"#;

        let info = monitor.analyze_log(log);
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.issue_type, KernelIssue::Panic);
        assert!(!info.stack_trace.is_empty());
    }

    #[test]
    fn test_check_line() {
        assert!(KernelMonitor::check_line("Kernel panic!").is_some());
        assert!(KernelMonitor::check_line("BUG: something bad").is_some());
        assert!(KernelMonitor::check_line("All good").is_none());
    }
}
