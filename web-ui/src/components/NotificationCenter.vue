<script setup lang="ts">
import { useNotificationStore } from '@/stores/notification'
import type { NotificationItem } from '@/types/notification'

const notificationStore = useNotificationStore()

function formatTime(date: Date): string {
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  const hours = Math.floor(diff / 3600000)

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (hours < 24) return `${hours}小时前`
  return date.toLocaleDateString()
}

function getTypeIcon(type: NotificationItem['type']): string {
  switch (type) {
    case 'success': return '✓'
    case 'error': return '✗'
    case 'warning': return '⚠'
    default: return 'ℹ'
  }
}

function getTypeClass(type: NotificationItem['type']): string {
  switch (type) {
    case 'success': return 'text-green-500'
    case 'error': return 'text-red-500'
    case 'warning': return 'text-yellow-500'
    default: return 'text-blue-500'
  }
}

function handleNotificationClick(notification: NotificationItem) {
  notificationStore.markAsRead(notification.id)
  // 如果有关联的任务，跳转到任务详情
  if (notification.taskId) {
    window.location.href = `/tasks/${notification.taskId}`
  }
  notificationStore.closeCenter()
}
</script>

<template>
  <!-- 通知中心面板 -->
  <div
    v-if="notificationStore.showCenter"
    class="fixed inset-0 z-50"
    @click="notificationStore.closeCenter"
  >
    <div
      class="absolute right-4 top-16 w-96 max-h-[80vh] bg-terminal-surface border border-terminal-border rounded-lg shadow-xl overflow-hidden"
      @click.stop
    >
      <!-- 头部 -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-terminal-border">
        <h3 class="font-semibold">通知中心</h3>
        <div class="flex gap-2">
          <button
            @click="notificationStore.markAllAsRead"
            class="text-xs text-terminal-muted hover:text-terminal-accent"
            :disabled="notificationStore.unreadCount === 0"
          >
            全部已读
          </button>
          <button
            @click="notificationStore.clearAll"
            class="text-xs text-terminal-muted hover:text-terminal-error"
            :disabled="notificationStore.notifications.length === 0"
          >
            清空
          </button>
        </div>
      </div>

      <!-- 通知列表 -->
      <div class="overflow-y-auto max-h-[60vh]">
        <div v-if="notificationStore.notifications.length === 0" class="p-8 text-center text-terminal-muted">
          暂无通知
        </div>
        <div
          v-for="notification in notificationStore.notifications"
          :key="notification.id"
          class="p-4 border-b border-terminal-border hover:bg-terminal-border/20 cursor-pointer transition-colors"
          :class="{ 'bg-terminal-accent/5': !notification.read }"
          @click="handleNotificationClick(notification)"
        >
          <div class="flex items-start gap-3">
            <span :class="getTypeClass(notification.type)" class="text-lg">
              {{ getTypeIcon(notification.type) }}
            </span>
            <div class="flex-1 min-w-0">
              <div class="font-medium text-sm">{{ notification.title }}</div>
              <div class="text-xs text-terminal-muted mt-1 line-clamp-2">{{ notification.message }}</div>
              <div class="text-xs text-terminal-muted mt-1">{{ formatTime(notification.timestamp) }}</div>
            </div>
            <div v-if="!notification.read" class="w-2 h-2 rounded-full bg-terminal-accent"></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
