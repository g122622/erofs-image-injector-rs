<script setup lang="ts">
import { onMounted, onUnmounted, ref, provide } from 'vue'
import { useTaskStore } from '@/stores/task'

const taskStore = useTaskStore()

// Notification state
const notification = ref<{ message: string; type: 'success' | 'error' } | null>(null)
let notificationTimer: ReturnType<typeof setTimeout> | null = null

function showNotification(message: string, type: 'success' | 'error') {
  notification.value = { message, type }
  if (notificationTimer) {
    clearTimeout(notificationTimer)
  }
  notificationTimer = setTimeout(() => {
    notification.value = null
  }, 3000)
}

let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

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
    case 'progress':
      taskStore.updateProgress(msg.task_id, msg.iteration, msg.crashes, msg.speed)
      break
    case 'status':
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
  taskStore.fetchTasks()
  taskStore.fetchStats()
  connectWebSocket()
})

onUnmounted(() => {
  ws?.close()
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
  }
  if (notificationTimer) {
    clearTimeout(notificationTimer)
  }
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
            </nav>
          </div>
          <div class="text-sm text-terminal-muted">
            <span class="inline-block w-2 h-2 rounded-full bg-terminal-success mr-2"></span>
            Connected
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="container mx-auto px-4 py-6">
      <router-view />
    </main>

    <!-- Notification -->
    <div
      v-if="notification"
      class="fixed bottom-4 right-4 z-50 px-4 py-3 rounded-md shadow-lg"
      :class="notification.type === 'success' ? 'bg-green-600 text-white' : 'bg-red-600 text-white'"
    >
      {{ notification.message }}
    </div>
  </div>
</template>
