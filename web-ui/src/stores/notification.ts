import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { NotificationItem, NotificationConfig, NotificationEvent, NotificationType } from '@/types/notification'
import { defaultNotificationConfig } from '@/types/notification'

export const useNotificationStore = defineStore('notification', () => {
  // 通知列表
  const notifications = ref<NotificationItem[]>([])

  // 配置
  const config = ref<NotificationConfig>({ ...defaultNotificationConfig })

  // 浏览器通知权限状态
  const browserPermission = ref<NotificationPermission>('default')

  // 未读数量
  const unreadCount = computed(() => notifications.value.filter(n => !n.read).length)

  // 是否显示通知中心
  const showCenter = ref(false)

  // 初始化
  function init() {
    // 加载配置
    const savedConfig = localStorage.getItem('notificationConfig')
    if (savedConfig) {
      try {
        config.value = { ...defaultNotificationConfig, ...JSON.parse(savedConfig) }
      } catch (e) {
        console.error('Failed to load notification config:', e)
      }
    } else {
      // 首次访问，请求浏览器通知权限
      if ('Notification' in window && Notification.permission === 'default') {
        requestBrowserPermission()
      }
    }

    // 检查浏览器通知权限
    if ('Notification' in window) {
      browserPermission.value = Notification.permission
    }
  }

  // 请求浏览器通知权限
  async function requestBrowserPermission(): Promise<boolean> {
    if (!('Notification' in window)) {
      console.warn('Browser does not support notifications')
      return false
    }

    try {
      const permission = await Notification.requestPermission()
      browserPermission.value = permission

      if (permission === 'granted') {
        // 显示欢迎通知
        new Notification('通知已启用', {
          body: '您将收到任务状态变化和崩溃发现的通知',
          icon: '/favicon.ico',
        })
      }

      return permission === 'granted'
    } catch (e) {
      console.error('Failed to request notification permission:', e)
      return false
    }
  }

  // 保存配置
  function saveConfig(newConfig: Partial<NotificationConfig>) {
    config.value = { ...config.value, ...newConfig }
    localStorage.setItem('notificationConfig', JSON.stringify(config.value))

    // 如果启用了浏览器通知，确保有权限
    if (newConfig.browserEnabled && browserPermission.value !== 'granted') {
      requestBrowserPermission()
    }
  }

  // 添加通知
  function addNotification(
    event: NotificationEvent,
    type: NotificationType,
    title: string,
    message: string,
    taskId?: number
  ) {
    // 检查该事件是否启用（如果事件不在配置中，默认启用）
    const eventEnabled = config.value.events[event]
    if (eventEnabled === false) {
      return
    }

    const id = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
    const notification: NotificationItem = {
      id,
      type,
      title,
      message,
      event,
      taskId,
      timestamp: new Date(),
      read: false,
    }

    notifications.value.unshift(notification)

    // 发送浏览器通知
    if (config.value.browserEnabled && browserPermission.value === 'granted') {
      try {
        new Notification(title, {
          body: message,
          icon: '/favicon.ico',
          tag: id,
        })
      } catch (e) {
        console.error('Failed to send browser notification:', e)
      }
    }

    // 显示Toast（通过事件通知App.vue）
    if (config.value.toastEnabled) {
      window.dispatchEvent(new CustomEvent('app-notification', {
        detail: { type, title, message }
      }))
    }

    return notification
  }

  // 标记为已读
  function markAsRead(id: string) {
    const notification = notifications.value.find(n => n.id === id)
    if (notification) {
      notification.read = true
    }
  }

  // 标记全部已读
  function markAllAsRead() {
    notifications.value.forEach(n => n.read = true)
  }

  // 清除所有通知
  function clearAll() {
    notifications.value = []
  }

  // 切换通知中心显示
  function toggleCenter() {
    showCenter.value = !showCenter.value
  }

  // 关闭通知中心
  function closeCenter() {
    showCenter.value = false
  }

  return {
    notifications,
    config,
    browserPermission,
    unreadCount,
    showCenter,
    init,
    requestBrowserPermission,
    saveConfig,
    addNotification,
    markAsRead,
    markAllAsRead,
    clearAll,
    toggleCenter,
    closeCenter,
  }
})
