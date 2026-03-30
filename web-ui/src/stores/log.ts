import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { LogLevel } from '@/types'

// 日志配置
export interface LogConfig {
  // 最大日志条数
  maxLogs: number
  // 是否自动滚动
  autoScroll: boolean
  // 过滤的日志级别
  levelFilter: LogLevel[]
}

// 日志条目
export interface LogEntry {
  id: string
  taskId: number
  level: LogLevel
  message: string
  timestamp: Date
}

const defaultConfig: LogConfig = {
  maxLogs: 1000,
  autoScroll: true,
  levelFilter: ['debug', 'info', 'warn', 'error'],
}

export const useLogStore = defineStore('log', () => {
  // 按任务分组的日志
  const logsByTask = ref<Map<number, LogEntry[]>>(new Map())

  // 配置
  const config = ref<LogConfig>({ ...defaultConfig })

  // 当前查看的任务ID
  const currentTaskId = ref<number | null>(null)

  // 当前任务的日志
  const currentLogs = computed(() => {
    if (!currentTaskId.value) return []
    return logsByTask.value.get(currentTaskId.value) || []
  })

  // 过滤后的日志
  const filteredLogs = computed(() => {
    return currentLogs.value.filter(log => config.value.levelFilter.includes(log.level))
  })

  // 初始化
  function init() {
    const savedConfig = localStorage.getItem('logConfig')
    if (savedConfig) {
      try {
        config.value = { ...defaultConfig, ...JSON.parse(savedConfig) }
      } catch (e) {
        console.error('Failed to load log config:', e)
      }
    }
  }

  // 保存配置
  function saveConfig(newConfig: Partial<LogConfig>) {
    config.value = { ...config.value, ...newConfig }
    localStorage.setItem('logConfig', JSON.stringify(config.value))
  }

  // 添加日志
  function addLog(taskId: number, level: LogLevel, message: string, timestamp: number) {
    const logs = logsByTask.value.get(taskId) || []

    const entry: LogEntry = {
      id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      taskId,
      level,
      message,
      timestamp: new Date(timestamp * 1000),
    }

    logs.push(entry)

    // 限制日志数量
    if (logs.length > config.value.maxLogs) {
      logs.splice(0, logs.length - config.value.maxLogs)
    }

    logsByTask.value.set(taskId, logs)
  }

  // 清除任务的日志
  function clearLogs(taskId?: number) {
    if (taskId) {
      logsByTask.value.delete(taskId)
    } else if (currentTaskId.value) {
      logsByTask.value.delete(currentTaskId.value)
    }
  }

  // 清除所有日志
  function clearAllLogs() {
    logsByTask.value.clear()
  }

  // 设置当前任务
  function setCurrentTask(taskId: number | null) {
    currentTaskId.value = taskId
  }

  // 切换日志级别过滤
  function toggleLevel(level: LogLevel) {
    const filter = config.value.levelFilter
    const index = filter.indexOf(level)
    if (index === -1) {
      filter.push(level)
    } else {
      filter.splice(index, 1)
    }
    saveConfig({ levelFilter: filter })
  }

  // 导出日志
  function exportLogs(taskId?: number): string {
    const logs = taskId
      ? (logsByTask.value.get(taskId) || [])
      : currentLogs.value

    return logs.map(log =>
      `[${log.timestamp.toISOString()}] [${log.level.toUpperCase()}] ${log.message}`
    ).join('\n')
  }

  return {
    logsByTask,
    config,
    currentTaskId,
    currentLogs,
    filteredLogs,
    init,
    saveConfig,
    addLog,
    clearLogs,
    clearAllLogs,
    setCurrentTask,
    toggleLevel,
    exportLogs,
  }
})
