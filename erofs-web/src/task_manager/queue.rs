//! Task queue implementation

use std::collections::VecDeque;

/// Task queue for managing pending tasks
#[derive(Debug, Default)]
pub struct TaskQueue {
    /// Queue of pending task IDs
    pending: VecDeque<i64>,
    /// Set of queued task IDs for quick lookup
    queued: std::collections::HashSet<i64>,
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            queued: std::collections::HashSet::new(),
        }
    }

    /// Add a task to the queue
    pub fn enqueue(&mut self, task_id: i64) {
        if !self.queued.contains(&task_id) {
            self.pending.push_back(task_id);
            self.queued.insert(task_id);
        }
    }

    /// Get the next task from the queue
    pub fn dequeue(&mut self) -> Option<i64> {
        let task_id = self.pending.pop_front();
        if let Some(id) = task_id {
            self.queued.remove(&id);
        }
        task_id
    }

    /// Remove a task from the queue
    pub fn remove(&mut self, task_id: i64) {
        self.pending.retain(|&id| id != task_id);
        self.queued.remove(&task_id);
    }

    /// Check if a task is in the queue
    pub fn contains(&self, task_id: i64) -> bool {
        self.queued.contains(&task_id)
    }

    /// Get the number of pending tasks
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Get all pending task IDs
    pub fn pending(&self) -> Vec<i64> {
        self.pending.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_operations() {
        let mut queue = TaskQueue::new();

        // Test enqueue
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        assert_eq!(queue.len(), 3);
        assert!(queue.contains(1));
        assert!(queue.contains(2));
        assert!(queue.contains(3));
        assert!(!queue.contains(4));

        // Test dequeue (FIFO)
        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.len(), 2);
        assert!(!queue.contains(1));

        // Test remove
        queue.remove(3);
        assert_eq!(queue.len(), 1);
        assert!(!queue.contains(3));

        // Test remaining
        assert_eq!(queue.dequeue(), Some(2));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_duplicate_enqueue() {
        let mut queue = TaskQueue::new();

        queue.enqueue(1);
        queue.enqueue(1); // Duplicate
        queue.enqueue(1); // Duplicate

        assert_eq!(queue.len(), 1);
    }
}
