<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useTaskStore } from '@/stores/task'
import { api } from '@/api'
import type { TaskConfig, StrategyTemplate } from '@/types'

const taskStore = useTaskStore()

const strategies = ref<StrategyTemplate[]>([])
const showStrategyGuide = ref(false)

const createDefaultForm = (): TaskConfig => ({
  name: '',
  executor_type: 'qemu',
  seeds_dir: './seeds',
  output_dir: './crashes_kernel',
  timeout_seconds: 300,
  max_iterations: 100,
  workers: 1,
  kernel_path: './kernel_build/bzImage',
  initramfs_path: './kernel_build/rootfs.cpio.gz',
  qemu_path: '/usr/bin/qemu-system-x86_64',
  qemu_memory: 1024,
  erofsfuse_path: 'erofsfuse',
  strategy_id: undefined,
})

const form = ref<TaskConfig>(createDefaultForm())

const submitting = ref(false)
const error = ref<string | null>(null)

const selectedStrategy = computed(() => {
  if (!form.value.strategy_id) return null
  return strategies.value.find(s => s.id === form.value.strategy_id)
})

onMounted(async () => {
  try {
    strategies.value = await api.listStrategies()
    // Show strategy guide if no strategy selected and strategies are loaded
    showStrategyGuide.value = strategies.value.length > 0 && !form.value.strategy_id
  } catch (e) {
    console.error('Failed to load strategies:', e)
  }
})

async function handleSubmit() {
  submitting.value = true
  error.value = null

  try {
    const task = await taskStore.createTask(form.value)
    // 自动启动任务
    await taskStore.startTask(task.id)
    // 重置表单
    form.value = createDefaultForm()
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to create task'
  } finally {
    submitting.value = false
  }
}

function selectStrategy(strategy: StrategyTemplate | null) {
  form.value.strategy_id = strategy?.id
  showStrategyGuide.value = false
}

function getEnabledMutatorCount(strategy: StrategyTemplate): number {
  return Object.values(strategy.mutators).filter(c => c.enabled).length
}
</script>

<template>
  <form @submit.prevent="handleSubmit" class="card p-4 space-y-4">
    <div v-if="error" class="bg-terminal-error/20 text-terminal-error px-3 py-2 rounded text-sm">
      {{ error }}
    </div>

    <!-- Strategy Selection Guide -->
    <div v-if="showStrategyGuide && strategies.length > 0" class="bg-terminal-surface border border-terminal-border rounded-lg p-4">
      <h3 class="text-sm font-medium text-terminal-text mb-3">Select a Mutation Strategy</h3>
      <p class="text-xs text-terminal-muted mb-4">Choose a strategy template for your fuzzing task.</p>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <button
          v-for="strategy in strategies.filter(s => s.is_builtin).slice(0, 4)"
          :key="strategy.id"
          type="button"
          @click="selectStrategy(strategy)"
          class="p-3 border rounded-lg text-left transition-colors"
          :class="form.strategy_id === strategy.id
            ? 'border-terminal-accent bg-terminal-accent/10'
            : 'border-terminal-border hover:border-terminal-accent/50'"
        >
          <div class="flex items-center justify-between mb-1">
            <span class="text-sm font-medium text-terminal-text">{{ strategy.name }}</span>
            <span class="text-xs text-terminal-muted">{{ getEnabledMutatorCount(strategy) }} mutators</span>
          </div>
          <p class="text-xs text-terminal-muted line-clamp-2">{{ strategy.description }}</p>
        </button>
        <button
          type="button"
          @click="selectStrategy(null)"
          class="p-3 border border-dashed border-terminal-border rounded-lg text-left hover:border-terminal-accent/50 transition-colors"
        >
          <div class="text-sm text-terminal-muted">No Strategy (Default)</div>
          <p class="text-xs text-terminal-muted mt-1">Use default mutation settings</p>
        </button>
      </div>
    </div>

    <!-- Selected Strategy Display -->
    <div v-else class="flex items-center justify-between p-3 bg-terminal-surface border border-terminal-border rounded-lg">
      <div>
        <span class="text-xs text-terminal-muted">Strategy:</span>
        <span class="ml-2 text-sm text-terminal-text">
          {{ selectedStrategy?.name || 'Default (No strategy)' }}
        </span>
        <span v-if="selectedStrategy" class="ml-2 text-xs text-terminal-muted">
          ({{ getEnabledMutatorCount(selectedStrategy) }} mutators)
        </span>
      </div>
      <button
        type="button"
        @click="showStrategyGuide = true"
        class="text-xs text-terminal-accent hover:text-terminal-accent/80"
      >
        Change
      </button>
    </div>

    <div>
      <label class="block text-sm text-terminal-muted mb-1">Task Name</label>
      <input
        v-model="form.name"
        type="text"
        placeholder="my-fuzz-task"
        class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        required
      />
    </div>

    <div>
      <label class="block text-sm text-terminal-muted mb-1">Executor Type</label>
      <select
        v-model="form.executor_type"
        class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
      >
        <option value="erofsfuse">erofsfuse (User-space)</option>
        <option value="qemu">QEMU (Kernel)</option>
      </select>
    </div>

    <div>
      <label class="block text-sm text-terminal-muted mb-1">Seeds Directory</label>
      <input
        v-model="form.seeds_dir"
        type="text"
        placeholder="./seeds"
        class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
      />
    </div>

    <div>
      <label class="block text-sm text-terminal-muted mb-1">Output Directory</label>
      <input
        v-model="form.output_dir"
        type="text"
        placeholder="./crashes"
        class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
      />
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div>
        <label class="block text-sm text-terminal-muted mb-1">Timeout (s)</label>
        <input
          v-model.number="form.timeout_seconds"
          type="number"
          min="1"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
      <div>
        <label class="block text-sm text-terminal-muted mb-1">Max Iterations</label>
        <input
          v-model.number="form.max_iterations"
          type="number"
          min="0"
          placeholder="0 = unlimited"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
    </div>

    <div v-if="form.executor_type === 'qemu'" class="space-y-4">
      <div>
        <label class="block text-sm text-terminal-muted mb-1">Kernel Path</label>
        <input
          v-model="form.kernel_path"
          type="text"
          placeholder="./kernel_build/bzImage"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
      <div>
        <label class="block text-sm text-terminal-muted mb-1">Initramfs Path</label>
        <input
          v-model="form.initramfs_path"
          type="text"
          placeholder="./kernel_build/rootfs.cpio.gz"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
      <div>
        <label class="block text-sm text-terminal-muted mb-1">QEMU Path</label>
        <input
          v-model="form.qemu_path"
          type="text"
          placeholder="/usr/bin/qemu-system-x86_64"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
      <div>
        <label class="block text-sm text-terminal-muted mb-1">QEMU Memory (MB)</label>
        <input
          v-model.number="form.qemu_memory"
          type="number"
          min="128"
          placeholder="1024"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
    </div>

    <div v-if="form.executor_type === 'erofsfuse'" class="space-y-4">
      <div>
        <label class="block text-sm text-terminal-muted mb-1">erofsfuse Path</label>
        <input
          v-model="form.erofsfuse_path"
          type="text"
          placeholder="erofsfuse"
          class="w-full bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
        />
      </div>
    </div>

    <button
      type="submit"
      :disabled="submitting"
      class="w-full btn btn-primary py-2"
    >
      {{ submitting ? 'Creating...' : 'Create & Start Task' }}
    </button>
  </form>
</template>
