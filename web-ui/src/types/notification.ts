// 通知类型
export type NotificationType = 'info' | 'success' | 'warning' | 'error'

// 通知场景
export type NotificationEvent =
  | 'task_completed'
  | 'task_failed'
  | 'task_started'
  | 'task_created'
  | 'new_crash'
  | 'crash_threshold'
  | 'seed_deleted'
  | 'seed_delete_failed'
  | 'seed_downloaded'
  | 'seed_download_failed'
  | 'seeds_generated'
  | 'seed_generate_failed'
  | 'seed_uploaded'
  | 'seed_upload_failed'
  | 'validation'

// 通知配置
export interface NotificationConfig {
  // 是否启用浏览器通知
  browserEnabled: boolean
  // 是否启用页面内通知
  toastEnabled: boolean
  // 各事件是否启用
  events: Record<NotificationEvent, boolean>
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
    task_created: true,
    new_crash: true,
    crash_threshold: false,
    seed_deleted: true,
    seed_delete_failed: true,
    seed_downloaded: true,
    seed_download_failed: true,
    seeds_generated: true,
    seed_generate_failed: true,
    seed_uploaded: true,
    seed_upload_failed: true,
    validation: true,
  },
  crashThreshold: 100,
}
