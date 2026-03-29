<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api'
import type { Task, Crash } from '@/types'

const route = useRoute()
const task = ref<Task | null>(null)
const crashes = ref<Crash[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

const taskId = computed(() => Number(route.params.id))

onMounted(async () => {
  try {
    task.value = await api.getTask(taskId.value)
    crashes.value = await api.listCrashes({ task_id: taskId.value, limit: 20 })
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load task'
  } finally {
    loading.value = false
  }
})

function getStatusClass(status: Task['status']): string {
  const classes: Record<Task['status'], string> = {
    pending: 'badge-pending',
    running: 'badge-running',
    paused: 'badge-paused',
    completed: 'badge-completed',
    failed: 'badge-failed',
    cancelled: 'badge-pending',
  }
  return classes[status] || 'badge-pending'
}

function formatDate(date: string): string {
  return new Date(date).toLocaleString()
}
</script>

<template>
  <div class="space-y-6">
    <div v-if="loading" class="text-center text-terminal-muted py-8">
      Loading...
    </div>

    <div v-else-if="error" class="card p-4 bg-terminal-error/20 text-terminal-error">
      {{ error }}
    </div>

    <template v-else-if="task">
      <div class="flex justify-between items-center">
        <div>
          <h1 class="text-2xl font-bold">{{ task.name }}</h1>
          <p class="text-terminal-muted">Task #{{ task.id }}</p>
        </div>
        <span :class="['badge', getStatusClass(task.status)]">{{ task.status }}</span>
      </div>

      <!-- Stats -->
      <div class="grid grid-cols-4 gap-4">
        <div class="card p-4">
          <div class="text-terminal-muted text-sm">Iterations</div>
          <div class="text-xl font-bold">{{ task.current_iteration.toLocaleString() }}</div>
        </div>
        <div class="card p-4">
          <div class="text-terminal-muted text-sm">Crashes</div>
          <div class="text-xl font-bold text-terminal-error">{{ task.total_crashes }}</div>
        </div>
        <div class="card p-4">
          <div class="text-terminal-muted text-sm">Speed</div>
          <div class="text-xl font-bold">{{ task.exec_per_sec.toFixed(1) }}/s</div>
        </div>
        <div class="card p-4">
          <div class="text-terminal-muted text-sm">Executor</div>
          <div class="text-xl font-bold">{{ task.executor_type }}</div>
        </div>
      </div>

      <!-- Configuration -->
      <div class="card p-4">
        <h2 class="text-lg font-semibold mb-4">Configuration</h2>
        <div class="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span class="text-terminal-muted">Seeds Dir:</span>
            <span class="ml-2">{{ task.seeds_dir }}</span>
          </div>
          <div>
            <span class="text-terminal-muted">Output Dir:</span>
            <span class="ml-2">{{ task.output_dir }}</span>
          </div>
          <div>
            <span class="text-terminal-muted">Timeout:</span>
            <span class="ml-2">{{ task.timeout_seconds }}s</span>
          </div>
          <div>
            <span class="text-terminal-muted">Max Iterations:</span>
            <span class="ml-2">{{ task.max_iterations || 'Unlimited' }}</span>
          </div>
          <div>
            <span class="text-terminal-muted">Workers:</span>
            <span class="ml-2">{{ task.workers }}</span>
          </div>
          <div v-if="task.qemu_memory">
            <span class="text-terminal-muted">QEMU Memory:</span>
            <span class="ml-2">{{ task.qemu_memory }} MB</span>
          </div>
          <div v-if="task.kernel_path">
            <span class="text-terminal-muted">Kernel:</span>
            <span class="ml-2">{{ task.kernel_path }}</span>
          </div>
          <div v-if="task.initramfs_path">
            <span class="text-terminal-muted">Initramfs:</span>
            <span class="ml-2">{{ task.initramfs_path }}</span>
          </div>
          <div v-if="task.qemu_path">
            <span class="text-terminal-muted">QEMU Path:</span>
            <span class="ml-2">{{ task.qemu_path }}</span>
          </div>
          <div v-if="task.erofsfuse_path">
            <span class="text-terminal-muted">erofsfuse Path:</span>
            <span class="ml-2">{{ task.erofsfuse_path }}</span>
          </div>
        </div>
      </div>

      <!-- Timeline -->
      <div class="card p-4">
        <h2 class="text-lg font-semibold mb-4">Timeline</h2>
        <div class="space-y-2 text-sm">
          <div class="flex justify-between">
            <span class="text-terminal-muted">Created</span>
            <span>{{ formatDate(task.created_at) }}</span>
          </div>
          <div v-if="task.started_at" class="flex justify-between">
            <span class="text-terminal-muted">Started</span>
            <span>{{ formatDate(task.started_at) }}</span>
          </div>
          <div v-if="task.finished_at" class="flex justify-between">
            <span class="text-terminal-muted">Finished</span>
            <span>{{ formatDate(task.finished_at) }}</span>
          </div>
          <div v-if="task.error_message" class="mt-2 p-2 bg-terminal-error/20 text-terminal-error rounded">
            {{ task.error_message }}
          </div>
        </div>
      </div>

      <!-- Crashes -->
      <div class="card p-4">
        <h2 class="text-lg font-semibold mb-4">Crashes ({{ crashes.length }})</h2>
        <div v-if="crashes.length > 0" class="space-y-2">
          <div
            v-for="crash in crashes"
            :key="crash.id"
            class="p-3 bg-terminal-bg rounded border border-terminal-border"
          >
            <div class="flex justify-between">
              <span class="font-medium text-terminal-error">{{ crash.crash_type }}</span>
              <span class="text-terminal-muted text-sm">Iteration {{ crash.iteration }}</span>
            </div>
            <div class="text-xs text-terminal-muted mt-1">{{ formatDate(crash.created_at) }}</div>
          </div>
        </div>
        <div v-else class="text-center text-terminal-muted py-4">
          No crashes recorded
        </div>
      </div>
    </template>
  </div>
</template>
