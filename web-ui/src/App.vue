<script setup lang="ts">
import { onMounted, onUnmounted, ref, provide } from 'vue'
import { useTaskStore } from '@/stores/task'
import { useNotificationStore } from '@/stores/notification'
import { useLogStore } from '@/stores/log'
import NotificationCenter from '@/components/NotificationCenter.vue'
import NotificationSettings from '@/components/NotificationSettings.vue'

const taskStore = useTaskStore()
const notificationStore = useNotificationStore()
const logStore = useLogStore()

// Toast notification state
const toast = ref<{ title: string; message: string; type: 'info' | 'success' | 'warning' | 'error' } | null>(null)
let toastTimer: ReturnType<typeof setTimeout> | null = null

function showToast(title: string, message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info') {
  toast.value = { title, message, type }
  if (toastTimer) {
    clearTimeout(toastTimer)
  }
  toastTimer = setTimeout(() => {
    toast.value = null
  }, 4000)
}

// Legacy notification for backward compatibility
function showNotification(message: string, type: 'success' | 'error') {
  showToast(type === 'success' ? '成功' : '错误', message, type)
}

// Handle custom notification events
function handleAppNotification(event: CustomEvent) {
  const { title, message, type } = event.detail
  showToast(title, message, type)
}

let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

// Track previous task states for notifications
const previousTaskStates = ref<Map<number, string>>(new Map())
// Track crash counts for notifications
const previousCrashCounts = ref<Map<number, number>>(new Map())

function connectWebSocket() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${window.location.host}/ws`)

  ws.onopen = () => {
    console.log('WebSocket connected')
    // 订阅所有任务
    ws?.send(JSON.stringify({ type: 'subscribe_all' }))
  }

  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data)
      handleWebSocketMessage(msg)
    } catch (e) {
      console.error('Failed to parse WebSocket message:', e)
    }
  }

  ws.onclose = () => {
    console.log('WebSocket disconnected, reconnecting...')
    reconnectTimer = setTimeout(connectWebSocket, 3000)
  }

  ws.onerror = (error) => {
    console.error('WebSocket error:', error)
  }
}

function handleWebSocketMessage(msg: any) {
  switch (msg.type) {
    case 'log':
      // 处理日志消息
      logStore.addLog(msg.task_id, msg.level, msg.message, msg.timestamp)
      break
    case 'progress':
      taskStore.updateProgress(msg.task_id, msg.iteration, msg.crashes, msg.speed)
      // Check for new crashes
      const prevCrashes = previousCrashCounts.value.get(msg.task_id) || 0
      if (msg.crashes > prevCrashes) {
        const task = taskStore.tasks.find(t => t.id === msg.task_id)
        notificationStore.addNotification(
          'new_crash',
          'info',
          '发现新崩溃',
          `任务 #${msg.task_id}${task ? ` (${task.name})` : ''} 发现新的崩溃，当前共 ${msg.crashes} 个`,
          msg.task_id
        )
      }
      previousCrashCounts.value.set(msg.task_id, msg.crashes)
      break
    case 'status':
      const prevState = previousTaskStates.value.get(msg.task_id)
      const task = taskStore.tasks.find(t => t.id === msg.task_id)

      // Send notification on status change
      if (prevState && prevState !== msg.status) {
        switch (msg.status) {
          case 'completed':
            notificationStore.addNotification(
              'task_completed',
              'success',
              '任务完成',
              `任务 #${msg.task_id}${task ? ` (${task.name})` : ''} 已完成`,
              msg.task_id
            )
            break
          case 'failed':
            notificationStore.addNotification(
              'task_failed',
              'error',
              '任务失败',
              `任务 #${msg.task_id}${task ? ` (${task.name})` : ''} 执行失败`,
              msg.task_id
            )
            break
          case 'running':
            if (prevState === 'pending') {
              notificationStore.addNotification(
                'task_started',
                'info',
                '任务启动',
                `任务 #${msg.task_id}${task ? ` (${task.name})` : ''} 已开始运行`,
                msg.task_id
              )
            }
            break
        }
      }

      previousTaskStates.value.set(msg.task_id, msg.status)
      taskStore.updateStatus(msg.task_id, msg.status)
      // 刷新任务详情
      taskStore.fetchTasks()
      break
    case 'crash':
      // 新崩溃，刷新统计
      taskStore.fetchStats()
      break
    case 'task_created':
      // 新任务，刷新列表
      taskStore.fetchTasks()
      break
  }
}

// Provide notification function globally
provide('notify', showNotification)

onMounted(() => {
  logStore.init()
  notificationStore.init()
  taskStore.fetchTasks()
  taskStore.fetchStats()
  connectWebSocket()

  // Listen for app notification events
  window.addEventListener('app-notification', handleAppNotification as EventListener)
})

onUnmounted(() => {
  ws?.close()
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
  }
  if (toastTimer) {
    clearTimeout(toastTimer)
  }
  window.removeEventListener('app-notification', handleAppNotification as EventListener)
})
</script>

<template>
  <div class="min-h-screen bg-terminal-bg text-terminal-text">
    <!-- Header -->
    <header class="border-b border-terminal-border bg-terminal-surface">
      <div class="container mx-auto px-4 py-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-4">
            <h1 class="text-xl font-bold text-terminal-accent">EROFS Fuzzer Console</h1>
            <nav class="flex gap-4">
              <router-link
                to="/"
                class="text-terminal-muted hover:text-terminal-text transition-colors"
                active-class="text-terminal-accent"
              >
                Dashboard
              </router-link>
              <router-link
                to="/tasks"
                class="text-terminal-muted hover:text-terminal-text transition-colors"
                active-class="text-terminal-accent"
              >
                Tasks
              </router-link>
              <router-link
                to="/crashes"
                class="text-terminal-muted hover:text-terminal-text transition-colors"
                active-class="text-terminal-accent"
              >
                Crashes
              </router-link>
              <router-link
                to="/strategies"
                class="text-terminal-muted hover:text-terminal-text transition-colors"
                active-class="text-terminal-accent"
              >
                Strategies
              </router-link>
              <router-link
                to="/seeds"
                class="text-terminal-muted hover:text-terminal-text transition-colors"
                active-class="text-terminal-accent"
              >
                Seeds
              </router-link>
            </nav>
          </div>
          <div class="flex items-center gap-4">
            <!-- 通知中心按钮 -->
            <button
              @click="notificationStore.toggleCenter"
              class="relative text-terminal-muted hover:text-terminal-text transition-colors"
              title="通知中心"
            >
              <span class="text-xl">🔔</span>
              <span
                v-if="notificationStore.unreadCount > 0"
                class="absolute -top-1 -right-1 w-4 h-4 bg-terminal-error text-white text-xs rounded-full flex items-center justify-center"
              >
                {{ notificationStore.unreadCount > 9 ? '9+' : notificationStore.unreadCount }}
              </span>
            </button>

            <!-- 通知设置 -->
            <NotificationSettings />

            <div class="text-sm text-terminal-muted">
              <span class="inline-block w-2 h-2 rounded-full bg-terminal-success mr-2"></span>
              Connected
            </div>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="container mx-auto px-4 py-6">
      <router-view />
    </main>

    <!-- Toast Notification -->
    <div
      v-if="toast"
      class="fixed bottom-4 right-4 z-50 max-w-sm bg-terminal-surface border border-terminal-border rounded-lg shadow-lg overflow-hidden"
      :class="{
        'border-green-500': toast.type === 'success',
        'border-red-500': toast.type === 'error',
        'border-yellow-500': toast.type === 'warning',
        'border-blue-500': toast.type === 'info',
      }"
    >
      <div class="p-4">
        <div class="font-semibold text-sm">{{ toast.title }}</div>
        <div class="text-xs text-terminal-muted mt-1">{{ toast.message }}</div>
      </div>
      <div class="h-1 bg-terminal-border">
        <div
          class="h-full transition-all duration-[4000ms] ease-linear"
          :class="{
            'bg-green-500': toast.type === 'success',
            'bg-red-500': toast.type === 'error',
            'bg-yellow-500': toast.type === 'warning',
            'bg-blue-500': toast.type === 'info',
          }"
          style="animation: shrink 4s linear forwards;"
        ></div>
      </div>
    </div>

    <!-- Notification Center -->
    <NotificationCenter />
  </div>
</template>

<style>
@keyframes shrink {
  from { width: 100%; }
  to { width: 0%; }
}
</style>
