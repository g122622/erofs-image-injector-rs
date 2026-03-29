<script setup lang="ts">
import type { Task } from '@/types'
import { useTaskStore } from '@/stores/task'
import { useRouter } from 'vue-router'

defineProps<{
  tasks: Task[]
}>()

const taskStore = useTaskStore()
const router = useRouter()

function goToDetail(id: number) {
  router.push(`/tasks/${id}`)
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

async function handleStart(id: number) {
  try {
    await taskStore.startTask(id)
  } catch (e) {
    console.error('Failed to start task:', e)
  }
}

async function handlePause(id: number) {
  try {
    await taskStore.pauseTask(id)
  } catch (e) {
    console.error('Failed to pause task:', e)
  }
}

async function handleResume(id: number) {
  try {
    await taskStore.resumeTask(id)
  } catch (e) {
    console.error('Failed to resume task:', e)
  }
}

async function handleStop(id: number) {
  try {
    await taskStore.stopTask(id)
  } catch (e) {
    console.error('Failed to stop task:', e)
  }
}

async function handleDelete(id: number) {
  if (confirm('Are you sure you want to delete this task?')) {
    try {
      await taskStore.deleteTask(id)
    } catch (e) {
      console.error('Failed to delete task:', e)
    }
  }
}
</script>

<template>
  <div class="card overflow-hidden">
    <table class="table-terminal" v-if="tasks.length > 0">
      <thead>
        <tr>
          <th>ID</th>
          <th>Name</th>
          <th>Status</th>
          <th>Executor</th>
          <th>Iterations</th>
          <th>Crashes</th>
          <th>Speed</th>
          <th>Created</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="task in tasks"
          :key="task.id"
          class="cursor-pointer"
          @click="goToDetail(task.id)"
        >
          <td>
            <router-link :to="`/tasks/${task.id}`" class="text-terminal-accent hover:underline" @click.stop>
              #{{ task.id }}
            </router-link>
          </td>
          <td>
            <router-link :to="`/tasks/${task.id}`" class="hover:text-terminal-accent" @click.stop>
              {{ task.name }}
            </router-link>
          </td>
          <td>
            <span :class="['badge', getStatusClass(task.status)]">{{ task.status }}</span>
          </td>
          <td>
            <span class="text-terminal-muted">{{ task.executor_type }}</span>
          </td>
          <td>{{ task.current_iteration.toLocaleString() }}</td>
          <td class="text-terminal-error">{{ task.total_crashes }}</td>
          <td>{{ task.exec_per_sec.toFixed(1) }}/s</td>
          <td class="text-terminal-muted text-xs">{{ formatDate(task.created_at) }}</td>
          <td>
            <div class="flex gap-1">
              <button
                @click.stop="goToDetail(task.id)"
                class="btn btn-secondary text-xs px-2 py-1"
              >
                Details
              </button>
              <button
                v-if="task.status === 'pending'"
                @click.stop="handleStart(task.id)"
                class="btn btn-primary text-xs px-2 py-1"
              >
                Start
              </button>
              <button
                v-if="task.status === 'running'"
                @click.stop="handlePause(task.id)"
                class="btn btn-secondary text-xs px-2 py-1"
              >
                Pause
              </button>
              <button
                v-if="task.status === 'paused'"
                @click.stop="handleResume(task.id)"
                class="btn btn-primary text-xs px-2 py-1"
              >
                Resume
              </button>
              <button
                v-if="task.status === 'running' || task.status === 'paused'"
                @click.stop="handleStop(task.id)"
                class="btn btn-danger text-xs px-2 py-1"
              >
                Stop
              </button>
              <button
                v-if="task.status !== 'running'"
                @click.stop="handleDelete(task.id)"
                class="btn btn-secondary text-xs px-2 py-1"
              >
                Delete
              </button>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
    <div v-else class="p-8 text-center text-terminal-muted">
      No tasks found. Create a new task to get started.
    </div>
  </div>
</template>
