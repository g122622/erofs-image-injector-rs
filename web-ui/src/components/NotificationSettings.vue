<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useNotificationStore } from '@/stores/notification'

const notificationStore = useNotificationStore()

onMounted(() => {
  notificationStore.init()
})

function toggleBrowserNotifications() {
  if (notificationStore.config.browserEnabled) {
    notificationStore.saveConfig({ browserEnabled: false })
  } else {
    notificationStore.requestBrowserPermission().then((granted) => {
      if (granted) {
        notificationStore.saveConfig({ browserEnabled: true })
      }
    })
  }
}

function toggleEvent(event: keyof typeof notificationStore.config.events) {
  notificationStore.saveConfig({
    events: {
      ...notificationStore.config.events,
      [event]: !notificationStore.config.events[event],
    },
  })
}

const showSettings = ref(false)
</script>

<template>
  <div class="relative">
    <!-- 设置按钮 -->
    <button
      @click="showSettings = !showSettings"
      class="text-terminal-muted hover:text-terminal-text transition-colors"
      title="通知设置"
    >
      ⚙
    </button>

    <!-- 设置面板 -->
    <div
      v-if="showSettings"
      class="absolute right-0 top-8 w-72 bg-terminal-surface border border-terminal-border rounded-lg shadow-xl z-50 p-4"
    >
      <h4 class="font-semibold mb-3">通知设置</h4>

      <!-- 浏览器通知 -->
      <div class="flex items-center justify-between py-2 border-b border-terminal-border">
        <div>
          <div class="text-sm">浏览器通知</div>
          <div class="text-xs text-terminal-muted">
            {{ notificationStore.browserPermission === 'granted' ? '已授权' : notificationStore.browserPermission === 'denied' ? '已拒绝' : '未授权' }}
          </div>
        </div>
        <button
          @click="toggleBrowserNotifications"
          :class="[
            'w-10 h-5 rounded-full transition-colors relative',
            notificationStore.config.browserEnabled ? 'bg-terminal-accent' : 'bg-terminal-border'
          ]"
          :disabled="notificationStore.browserPermission === 'denied'"
        >
          <span
            :class="[
              'absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform',
              notificationStore.config.browserEnabled ? 'left-5' : 'left-0.5'
            ]"
          ></span>
        </button>
      </div>

      <!-- Toast通知 -->
      <div class="flex items-center justify-between py-2 border-b border-terminal-border">
        <div class="text-sm">页面内通知</div>
        <button
          @click="notificationStore.saveConfig({ toastEnabled: !notificationStore.config.toastEnabled })"
          :class="[
            'w-10 h-5 rounded-full transition-colors relative',
            notificationStore.config.toastEnabled ? 'bg-terminal-accent' : 'bg-terminal-border'
          ]"
        >
          <span
            :class="[
              'absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform',
              notificationStore.config.toastEnabled ? 'left-5' : 'left-0.5'
            ]"
          ></span>
        </button>
      </div>

      <!-- 事件设置 -->
      <div class="mt-3">
        <div class="text-sm text-terminal-muted mb-2">通知事件</div>
        <div class="space-y-2">
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              :checked="notificationStore.config.events.task_completed"
              @change="toggleEvent('task_completed')"
              class="w-4 h-4 rounded border-terminal-border bg-terminal-bg accent-terminal-accent"
            />
            <span class="text-sm">任务完成</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              :checked="notificationStore.config.events.task_failed"
              @change="toggleEvent('task_failed')"
              class="w-4 h-4 rounded border-terminal-border bg-terminal-bg accent-terminal-accent"
            />
            <span class="text-sm">任务失败</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              :checked="notificationStore.config.events.task_started"
              @change="toggleEvent('task_started')"
              class="w-4 h-4 rounded border-terminal-border bg-terminal-bg accent-terminal-accent"
            />
            <span class="text-sm">任务启动</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              :checked="notificationStore.config.events.new_crash"
              @change="toggleEvent('new_crash')"
              class="w-4 h-4 rounded border-terminal-border bg-terminal-bg accent-terminal-accent"
            />
            <span class="text-sm">发现新崩溃</span>
          </label>
        </div>
      </div>

      <button
        @click="showSettings = false"
        class="w-full mt-4 btn btn-secondary text-sm"
      >
        关闭
      </button>
    </div>
  </div>
</template>
