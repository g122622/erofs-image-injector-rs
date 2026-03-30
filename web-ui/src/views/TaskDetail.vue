<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api'
import type { Task, Crash, StrategyTemplate, MutatorStats } from '@/types'
import { MUTATOR_INFO } from '@/types'
import LogPanel from '@/components/LogPanel.vue'

const route = useRoute()
const task = ref<Task | null>(null)
const crashes = ref<Crash[]>([])
const strategy = ref<StrategyTemplate | null>(null)
const mutatorStats = ref<MutatorStats[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

const taskId = computed(() => Number(route.params.id))

let ws: WebSocket | null = null

onMounted(async () => {
  try {
    task.value = await api.getTask(taskId.value)
    crashes.value = await api.listCrashes({ task_id: taskId.value, limit: 20 })

    // Load strategy if task has one
    if (task.value.strategy_id) {
      try {
        strategy.value = await api.getStrategy(task.value.strategy_id)
      } catch (e) {
        console.error('Failed to load strategy:', e)
      }
    }

    // Connect to WebSocket for real-time updates
    connectWebSocket()
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load task'
  } finally {
    loading.value = false
  }
})

onUnmounted(() => {
  ws?.close()
})

function connectWebSocket() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${protocol}//${window.location.host}/ws`)

  ws.onopen = () => {
    ws?.send(JSON.stringify({ type: 'subscribe', task_id: taskId.value }))
  }

  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data)
      console.log('WebSocket message:', msg)
      if (msg.type === 'strategy_stats' && msg.task_id === taskId.value) {
        mutatorStats.value = msg.stats.mutators
      } else if (msg.type === 'progress' && msg.task_id === taskId.value) {
        if (task.value) {
          task.value.current_iteration = msg.iteration
          task.value.total_crashes = msg.crashes
          task.value.exec_per_sec = msg.speed
          if (msg.current_mutator) {
            console.log('Received current_mutator:', msg.current_mutator)
            task.value.current_mutator = msg.current_mutator
          }
        }
      } else if (msg.type === 'status' && msg.task_id === taskId.value) {
        if (task.value) {
          task.value.status = msg.status
        }
      }
    } catch (e) {
      console.error('Failed to parse WebSocket message:', e)
    }
  }
}

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

function getMutatorName(mutator: string): string {
  return MUTATOR_INFO[mutator as keyof typeof MUTATOR_INFO]?.name || mutator
}

function getMutatorCategory(mutator: string): string {
  return MUTATOR_INFO[mutator as keyof typeof MUTATOR_INFO]?.category || 'basic'
}

function getCrashRate(stat: MutatorStats): string {
  if (stat.executions === 0) return '0.00%'
  return ((stat.crashes / stat.executions) * 100).toFixed(2) + '%'
}

function getWeightPercent(stat: MutatorStats): string {
  // Calculate from current_weight relative to total
  const total = mutatorStats.value.reduce((sum, s) => sum + s.current_weight, 0)
  if (total === 0) return '0%'
  return ((stat.current_weight / total) * 100).toFixed(1) + '%'
}

const totalIterations = computed(() => {
  return mutatorStats.value.reduce((sum, s) => sum + s.executions, 0)
})

const totalCrashesFromStats = computed(() => {
  return mutatorStats.value.reduce((sum, s) => sum + s.crashes, 0)
})
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
      <div class="grid grid-cols-5 gap-4">
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
        <div class="card p-4">
          <div class="text-terminal-muted text-sm">Current Mutator</div>
          <div class="text-xl font-bold text-terminal-accent">
            {{ task.current_mutator ? getMutatorName(task.current_mutator) : '-' }}
          </div>
        </div>
      </div>

      <!-- Version Info -->
      <div v-if="task.kernel_version || task.erofs_version" class="card p-4">
        <h2 class="text-lg font-semibold mb-3">Version Information</h2>
        <div class="grid grid-cols-2 gap-4 text-sm">
          <div v-if="task.kernel_version">
            <span class="text-terminal-muted">Kernel Version:</span>
            <span class="ml-2 font-mono text-terminal-accent">{{ task.kernel_version }}</span>
          </div>
          <div v-if="task.erofs_version">
            <span class="text-terminal-muted">EROFS Version:</span>
            <span class="ml-2 font-mono text-terminal-accent">{{ task.erofs_version }}</span>
          </div>
        </div>
      </div>

      <!-- Strategy Info -->
      <div v-if="strategy" class="card p-4">
        <div class="flex justify-between items-center mb-4">
          <h2 class="text-lg font-semibold">Strategy: {{ strategy.name }}</h2>
          <span v-if="strategy.adaptive_enabled" class="px-2 py-1 text-xs rounded bg-indigo-600 text-white">
            Adaptive
          </span>
        </div>
        <p v-if="strategy.description" class="text-sm text-terminal-muted mb-4">{{ strategy.description }}</p>

        <!-- Mutator Stats -->
        <div v-if="mutatorStats.length > 0" class="mt-4">
          <h3 class="text-sm font-medium text-terminal-muted mb-3">Mutator Statistics</h3>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-terminal-border">
                  <th class="text-left py-2 text-terminal-muted">Mutator</th>
                  <th class="text-right py-2 text-terminal-muted">Executions</th>
                  <th class="text-right py-2 text-terminal-muted">Crashes</th>
                  <th class="text-right py-2 text-terminal-muted">Crash Rate</th>
                  <th class="text-right py-2 text-terminal-muted">Weight</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="stat in mutatorStats"
                  :key="stat.mutator"
                  class="border-b border-terminal-border/50"
                >
                  <td class="py-2">
                    <div class="flex items-center gap-2">
                      <span>{{ getMutatorName(stat.mutator) }}</span>
                      <span
                        :class="[
                          'px-1.5 py-0.5 text-xs rounded',
                          getMutatorCategory(stat.mutator) === 'basic' ? 'bg-gray-600 text-gray-100' :
                          getMutatorCategory(stat.mutator) === 'structure' ? 'bg-blue-600 text-blue-100' :
                          'bg-purple-600 text-purple-100'
                        ]"
                      >
                        {{ getMutatorCategory(stat.mutator) }}
                      </span>
                    </div>
                  </td>
                  <td class="text-right py-2 font-mono">{{ stat.executions.toLocaleString() }}</td>
                  <td class="text-right py-2 font-mono text-terminal-error">{{ stat.crashes }}</td>
                  <td class="text-right py-2 font-mono">{{ getCrashRate(stat) }}</td>
                  <td class="text-right py-2 font-mono">{{ getWeightPercent(stat) }}</td>
                </tr>
              </tbody>
              <tfoot>
                <tr class="font-medium">
                  <td class="py-2">Total</td>
                  <td class="text-right py-2 font-mono">{{ totalIterations.toLocaleString() }}</td>
                  <td class="text-right py-2 font-mono text-terminal-error">{{ totalCrashesFromStats }}</td>
                  <td class="text-right py-2 font-mono">
                    {{ totalIterations > 0 ? ((totalCrashesFromStats / totalIterations) * 100).toFixed(2) + '%' : '0%' }}
                  </td>
                  <td class="text-right py-2 font-mono">100%</td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>

        <!-- Enabled Mutators (when no stats yet) -->
        <div v-else class="mt-4">
          <h3 class="text-sm font-medium text-terminal-muted mb-3">Enabled Mutators</h3>
          <div class="flex flex-wrap gap-2">
            <span
              v-for="(config, mutator) in strategy.mutators"
              :key="mutator"
              v-show="config.enabled"
              :class="[
                'px-2 py-1 text-xs rounded',
                getMutatorCategory(mutator as string) === 'basic' ? 'bg-gray-600 text-gray-100' :
                getMutatorCategory(mutator as string) === 'structure' ? 'bg-blue-600 text-blue-100' :
                'bg-purple-600 text-purple-100'
              ]"
            >
              {{ getMutatorName(mutator as string) }} ({{ config.weight }})
            </span>
          </div>
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
          <div v-if="task.strategy_id && !strategy">
            <span class="text-terminal-muted">Strategy ID:</span>
            <span class="ml-2">{{ task.strategy_id }}</span>
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

      <!-- Crashes -->
      <div class="card p-4">
        <h2 class="text-lg font-semibold mb-4">Crashes ({{ crashes.length }})</h2>
        <div v-if="crashes.length > 0" class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-terminal-border">
                <th class="text-left py-2 text-terminal-muted">ID</th>
                <th class="text-left py-2 text-terminal-muted">Type</th>
                <th class="text-left py-2 text-terminal-muted">Iteration</th>
                <th class="text-left py-2 text-terminal-muted">Time</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="crash in crashes"
                :key="crash.id"
                class="border-b border-terminal-border/50 hover:bg-terminal-border/20"
              >
                <td class="py-2 text-terminal-muted">#{{ crash.id }}</td>
                <td class="py-2">
                  <span
                    :class="[
                      'px-2 py-0.5 text-xs rounded',
                      crash.crash_type === 'KernelPanic' ? 'bg-red-500/20 text-red-400' :
                      crash.crash_type === 'KernelOops' ? 'bg-orange-500/20 text-orange-400' :
                      crash.crash_type === 'ASan' ? 'bg-yellow-500/20 text-yellow-400' :
                      'bg-red-500/20 text-red-400'
                    ]"
                  >
                    {{ crash.crash_type }}
                  </span>
                </td>
                <td class="py-2 text-left font-mono">{{ crash.iteration.toLocaleString() }}</td>
                <td class="py-2 text-terminal-muted text-xs">{{ formatDate(crash.created_at) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="text-center text-terminal-muted py-4">
          No crashes recorded
        </div>
      </div>

      <!-- Real-time Logs -->
      <div class="card p-4">
        <LogPanel :task-id="task.id" />
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
    </template>
  </div>
</template>
