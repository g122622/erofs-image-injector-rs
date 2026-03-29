import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { Task, TaskConfig, TaskStats } from '@/types'

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const stats = ref<TaskStats | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // 获取任务列表
  async function fetchTasks() {
    loading.value = true
    error.value = null
    try {
      tasks.value = await api.listTasks()
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch tasks'
    } finally {
      loading.value = false
    }
  }

  // 获取统计信息
  async function fetchStats() {
    try {
      stats.value = await api.getStats()
    } catch (e) {
      console.error('Failed to fetch stats:', e)
    }
  }

  // 创建任务
  async function createTask(config: TaskConfig) {
    loading.value = true
    error.value = null
    try {
      const task = await api.createTask(config)
      tasks.value.push(task)
      return task
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to create task'
      throw e
    } finally {
      loading.value = false
    }
  }

  // 启动任务
  async function startTask(id: number) {
    try {
      const updated = await api.startTask(id)
      const index = tasks.value.findIndex(t => t.id === id)
      if (index !== -1) {
        tasks.value[index] = updated
      }
      return updated
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to start task'
      throw e
    }
  }

  // 停止任务
  async function stopTask(id: number) {
    try {
      const updated = await api.stopTask(id)
      const index = tasks.value.findIndex(t => t.id === id)
      if (index !== -1) {
        tasks.value[index] = updated
      }
      return updated
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to stop task'
      throw e
    }
  }

  // 暂停任务
  async function pauseTask(id: number) {
    try {
      const updated = await api.pauseTask(id)
      const index = tasks.value.findIndex(t => t.id === id)
      if (index !== -1) {
        tasks.value[index] = updated
      }
      return updated
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to pause task'
      throw e
    }
  }

  // 恢复任务
  async function resumeTask(id: number) {
    try {
      const updated = await api.resumeTask(id)
      const index = tasks.value.findIndex(t => t.id === id)
      if (index !== -1) {
        tasks.value[index] = updated
      }
      return updated
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to resume task'
      throw e
    }
  }

  // 删除任务
  async function deleteTask(id: number) {
    try {
      await api.deleteTask(id)
      tasks.value = tasks.value.filter(t => t.id !== id)
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to delete task'
      throw e
    }
  }

  // 更新任务（从 WebSocket 消息）
  function updateTask(task: Task) {
    const index = tasks.value.findIndex(t => t.id === task.id)
    if (index !== -1) {
      tasks.value[index] = task
    } else {
      tasks.value.push(task)
    }
  }

  // 更新进度
  function updateProgress(taskId: number, iteration: number, crashes: number, speed: number) {
    const task = tasks.value.find(t => t.id === taskId)
    if (task) {
      task.current_iteration = iteration
      task.total_crashes = crashes
      task.exec_per_sec = speed
    }
  }

  // 更新状态
  function updateStatus(taskId: number, status: Task['status']) {
    const task = tasks.value.find(t => t.id === taskId)
    if (task) {
      task.status = status
    }
  }

  return {
    tasks,
    stats,
    loading,
    error,
    fetchTasks,
    fetchStats,
    createTask,
    startTask,
    stopTask,
    pauseTask,
    resumeTask,
    deleteTask,
    updateTask,
    updateProgress,
    updateStatus,
  }
})
