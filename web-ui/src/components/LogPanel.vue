<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useLogStore } from '@/stores/log'
import type { LogLevel } from '@/types'

const props = defineProps<{
  taskId: number
}>()

const logStore = useLogStore()

// 日志容器引用
const logContainer = ref<HTMLElement | null>(null)

// 是否暂停滚动
const isPaused = ref(false)

// 搜索关键词
const searchKeyword = ref('')

// 是否显示设置面板
const showSettings = ref(false)

// 过滤后的日志
const displayLogs = computed(() => {
  let logs = logStore.filteredLogs

  if (searchKeyword.value) {
    const keyword = searchKeyword.value.toLowerCase()
    logs = logs.filter(log => log.message.toLowerCase().includes(keyword))
  }

  return logs
})

// 日志级别颜色
function getLevelColor(level: LogLevel): string {
  switch (level) {
    case 'error': return 'text-red-500'
    case 'warn': return 'text-yellow-500'
    case 'debug': return 'text-gray-500'
    default: return 'text-green-400'
  }
}

// 日志级别背景色
function getLevelBg(level: LogLevel): string {
  switch (level) {
    case 'error': return 'bg-red-500/10'
    case 'warn': return 'bg-yellow-500/10'
    case 'debug': return 'bg-gray-500/10'
    default: return 'bg-green-500/10'
  }
}

// 格式化时间
function formatTime(date: Date): string {
  return date.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

// 自动滚动到底部
function scrollToBottom() {
  if (!logContainer.value || isPaused.value || !logStore.config.autoScroll) return

  nextTick(() => {
    logContainer.value?.scrollTo({
      top: logContainer.value?.scrollHeight || 0,
      behavior: 'smooth',
    })
  })
}

// 监听日志变化自动滚动
watch(() => logStore.filteredLogs.length, () => {
  scrollToBottom()
})

// 切换暂停
function togglePause() {
  isPaused.value = !isPaused.value
}

// 清除日志
function clearLogs() {
  logStore.clearLogs(props.taskId)
}

// 导出日志
function exportLogs() {
  const content = logStore.exportLogs(props.taskId)
  const blob = new Blob([content], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `task-${props.taskId}-logs-${new Date().toISOString().slice(0, 10)}.txt`
  a.click()
  URL.revokeObjectURL(url)
}

// 切换日志级别
function toggleLevel(level: LogLevel) {
  logStore.toggleLevel(level)
}

// 检查级别是否启用
function isLevelEnabled(level: LogLevel): boolean {
  return logStore.config.levelFilter.includes(level)
}

// 设置最大日志数
function setMaxLogs(count: number) {
  logStore.saveConfig({ maxLogs: count })
}

// 设置当前任务ID
onMounted(() => {
  logStore.setCurrentTask(props.taskId)
  logStore.init()
})

onUnmounted(() => {
  logStore.setCurrentTask(null)
})
</script>

<template>
  <div class="flex flex-col h-96 bg-terminal-surface border border-terminal-border rounded-lg overflow-hidden">
    <!-- 工具栏 -->
    <div class="flex items-center gap-2 px-3 py-2 border-b border-terminal-border bg-terminal-bg/50">
      <span class="text-sm font-semibold">实时日志</span>

      <div class="flex-1" />

      <!-- 搜索框 -->
      <div class="relative">
        <input
          v-model="searchKeyword"
          type="text"
          placeholder="搜索..."
          class="w-40 px-2 py-1 text-xs bg-terminal-bg border border-terminal-border rounded focus:outline-none focus:border-terminal-accent"
        />
      </div>

      <!-- 日志级别过滤 -->
      <div class="flex gap-1">
        <button
          @click="toggleLevel('debug')"
          :class="[
            'px-2 py-0.5 text-xs rounded transition-colors',
            isLevelEnabled('debug') ? 'bg-gray-500/20 text-gray-400' : 'bg-gray-500/5 text-gray-600'
          ]"
        >
          DEBUG
        </button>
        <button
          @click="toggleLevel('info')"
          :class="[
            'px-2 py-0.5 text-xs rounded transition-colors',
            isLevelEnabled('info') ? 'bg-green-500/20 text-green-400' : 'bg-green-500/5 text-green-600'
          ]"
        >
          INFO
        </button>
        <button
          @click="toggleLevel('warn')"
          :class="[
            'px-2 py-0.5 text-xs rounded transition-colors',
            isLevelEnabled('warn') ? 'bg-yellow-500/20 text-yellow-400' : 'bg-yellow-500/5 text-yellow-600'
          ]"
        >
          WARN
        </button>
        <button
          @click="toggleLevel('error')"
          :class="[
            'px-2 py-0.5 text-xs rounded transition-colors',
            isLevelEnabled('error') ? 'bg-red-500/20 text-red-400' : 'bg-red-500/5 text-red-600'
          ]"
        >
          ERROR
        </button>
      </div>

      <!-- 操作按钮 -->
      <div class="flex gap-1 ml-2">
        <button
          @click="togglePause"
          :class="[
            'px-2 py-1 text-xs rounded transition-colors',
            isPaused ? 'bg-yellow-500/20 text-yellow-400' : 'bg-terminal-border text-terminal-muted'
          ]"
          :title="isPaused ? '继续滚动' : '暂停滚动'"
        >
          {{ isPaused ? '▶ 继续' : '⏸ 暂停' }}
        </button>
        <button
          @click="clearLogs"
          class="px-2 py-1 text-xs rounded bg-terminal-border text-terminal-muted hover:text-terminal-text transition-colors"
          title="清除日志"
        >
          清除
        </button>
        <button
          @click="exportLogs"
          class="px-2 py-1 text-xs rounded bg-terminal-border text-terminal-muted hover:text-terminal-text transition-colors"
          title="导出日志"
        >
          导出
        </button>
        <button
          @click="showSettings = !showSettings"
          class="px-2 py-1 text-xs rounded bg-terminal-border text-terminal-muted hover:text-terminal-text transition-colors"
          title="设置"
        >
          ⚙
        </button>
      </div>
    </div>

    <!-- 设置面板 -->
    <div v-if="showSettings" class="px-3 py-2 border-b border-terminal-border bg-terminal-bg/30">
      <div class="flex items-center gap-4 text-xs">
        <div class="flex items-center gap-2">
          <span class="text-terminal-muted">最大日志数:</span>
          <select
            :value="logStore.config.maxLogs"
            @change="setMaxLogs(Number(($event.target as HTMLSelectElement).value))"
            class="bg-terminal-bg border border-terminal-border rounded px-2 py-1"
          >
            <option :value="500">500</option>
            <option :value="1000">1000</option>
            <option :value="2000">2000</option>
            <option :value="5000">5000</option>
          </select>
        </div>
        <label class="flex items-center gap-1">
          <input
            type="checkbox"
            :checked="logStore.config.autoScroll"
            @change="logStore.saveConfig({ autoScroll: ($event.target as HTMLInputElement).checked })"
            class="w-3 h-3"
          />
          <span>自动滚动</span>
        </label>
      </div>
    </div>

    <!-- 日志列表 -->
    <div
      ref="logContainer"
      class="flex-1 overflow-y-auto font-mono text-xs p-2 space-y-0.5"
    >
      <div v-if="displayLogs.length === 0" class="text-center text-terminal-muted py-8">
        暂无日志
      </div>
      <div
        v-for="log in displayLogs"
        :key="log.id"
        :class="[
          'flex items-start gap-2 px-2 py-0.5 rounded hover:bg-terminal-border/20',
          getLevelBg(log.level)
        ]"
      >
        <span class="text-terminal-muted shrink-0">{{ formatTime(log.timestamp) }}</span>
        <span :class="['shrink-0 font-bold uppercase', getLevelColor(log.level)]">
          [{{ log.level }}]
        </span>
        <span class="break-all whitespace-pre-wrap">{{ log.message }}</span>
      </div>
    </div>

    <!-- 状态栏 -->
    <div class="flex items-center gap-2 px-3 py-1 border-t border-terminal-border bg-terminal-bg/50 text-xs text-terminal-muted">
      <span>共 {{ logStore.currentLogs.length }} 条日志</span>
      <span>|</span>
      <span>显示 {{ displayLogs.length }} 条</span>
      <span v-if="isPaused" class="text-yellow-500">| 已暂停</span>
    </div>
  </div>
</template>
