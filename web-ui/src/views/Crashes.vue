<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api'
import type { Crash, CrashType } from '@/types'

const crashes = ref<Crash[]>([])
const loading = ref(true)
const filterTaskId = ref<number | null>(null)
const filterType = ref<CrashType | null>(null)

onMounted(async () => {
  await fetchCrashes()
})

async function fetchCrashes() {
  loading.value = true
  try {
    crashes.value = await api.listCrashes({
      task_id: filterTaskId.value ?? undefined,
      crash_type: filterType.value ?? undefined,
      limit: 50,
    })
  } catch (e) {
    console.error('Failed to fetch crashes:', e)
  } finally {
    loading.value = false
  }
}

function formatDate(date: string): string {
  return new Date(date).toLocaleString()
}

function getCrashTypeClass(type: CrashType): string {
  const classes: Record<CrashType, string> = {
    Signal: 'bg-terminal-warning/20 text-terminal-warning',
    ASan: 'bg-terminal-error/20 text-terminal-error',
    KernelPanic: 'bg-terminal-error/20 text-terminal-error',
    KernelOops: 'bg-terminal-warning/20 text-terminal-warning',
  }
  return classes[type] || ''
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex justify-between items-center">
      <h1 class="text-2xl font-bold">Crashes</h1>
      <button @click="fetchCrashes" class="btn btn-secondary">Refresh</button>
    </div>

    <!-- Filters -->
    <div class="card p-4">
      <div class="flex gap-4">
        <div>
          <label class="block text-sm text-terminal-muted mb-1">Task ID</label>
          <input
            v-model.number="filterTaskId"
            type="number"
            placeholder="All tasks"
            class="bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
          />
        </div>
        <div>
          <label class="block text-sm text-terminal-muted mb-1">Crash Type</label>
          <select
            v-model="filterType"
            class="bg-terminal-bg border border-terminal-border rounded px-3 py-2 text-sm focus:outline-none focus:border-terminal-accent"
          >
            <option :value="null">All types</option>
            <option value="Signal">Signal</option>
            <option value="ASan">ASan</option>
            <option value="KernelPanic">Kernel Panic</option>
            <option value="KernelOops">Kernel Oops</option>
          </select>
        </div>
        <div class="flex items-end">
          <button @click="fetchCrashes" class="btn btn-primary">Apply Filter</button>
        </div>
      </div>
    </div>

    <!-- Crashes List -->
    <div v-if="loading" class="text-center text-terminal-muted py-8">
      Loading...
    </div>

    <div v-else-if="crashes.length === 0" class="card p-8 text-center text-terminal-muted">
      No crashes found
    </div>

    <div v-else class="card overflow-hidden">
      <table class="table-terminal">
        <thead>
          <tr>
            <th>ID</th>
            <th>Task</th>
            <th>Type</th>
            <th>Iteration</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="crash in crashes" :key="crash.id">
            <td>#{{ crash.id }}</td>
            <td>
              <router-link :to="`/tasks/${crash.task_id}`" class="text-terminal-accent hover:underline">
                Task #{{ crash.task_id }}
              </router-link>
            </td>
            <td>
              <span :class="['badge', getCrashTypeClass(crash.crash_type)]">
                {{ crash.crash_type }}
              </span>
            </td>
            <td>{{ crash.iteration }}</td>
            <td class="text-terminal-muted text-xs">{{ formatDate(crash.created_at) }}</td>
            <td>
              <a
                :href="`/api/crashes/${crash.id}/image`"
                class="btn btn-secondary text-xs px-2 py-1 mr-1"
                download
              >
                Download
              </a>
              <a
                :href="`/api/crashes/${crash.id}/repro`"
                class="btn btn-secondary text-xs px-2 py-1"
              >
                Repro
              </a>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
