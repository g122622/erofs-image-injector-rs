// 通知类型
export type NotificationType = 'info' | 'success' | 'warning' | 'error'

// 通知场景
export type NotificationEvent =
  | 'task_completed'
  | 'task_failed'
  | 'task_started'
  | 'new_crash'
  | 'crash_threshold'

// 通知配置
export interface NotificationConfig {
  // 是否启用浏览器通知
  browserEnabled: boolean
  // 是否启用页面内通知
  toastEnabled: boolean
  // 各事件是否启用
  events: {
    task_completed: boolean
    task_failed: boolean
    task_started: boolean
    new_crash: boolean
    crash_threshold: boolean
  }
  // crash阈值（当达到此数量时通知）
  crashThreshold: number
}

// 通知项
export interface NotificationItem {
  id: string
  type: NotificationType
  title: string
  message: string
  event: NotificationEvent
  taskId?: number
  timestamp: Date
  read: boolean
}

// 默认配置
export const defaultNotificationConfig: NotificationConfig = {
  browserEnabled: false,
  toastEnabled: true,
  events: {
    task_completed: true,
    task_failed: true,
    task_started: false,
    new_crash: true,
    crash_threshold: false,
  },
  crashThreshold: 100,
}
