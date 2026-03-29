<script setup lang="ts">
import { ref } from 'vue'
import { useTaskStore } from '@/stores/task'
import type { TaskConfig } from '@/types'

const taskStore = useTaskStore()

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
})

const form = ref<TaskConfig>(createDefaultForm())

const submitting = ref(false)
const error = ref<string | null>(null)

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
</script>

<template>
  <form @submit.prevent="handleSubmit" class="card p-4 space-y-4">
    <div v-if="error" class="bg-terminal-error/20 text-terminal-error px-3 py-2 rounded text-sm">
      {{ error }}
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
