<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Task } from '@/types'
import { useTaskStore } from '@/stores/task'
import { useRouter } from 'vue-router'

const props = defineProps<{
  tasks: Task[]
}>()

const emit = defineEmits<{
  (e: 'batch-stop', ids: number[]): void
  (e: 'batch-delete', ids: number[]): void
}>()

const taskStore = useTaskStore()
const router = useRouter()

// 批量选择相关
const selectedIds = ref<Set<number>>(new Set())
const showConfirmModal = ref(false)
const confirmAction = ref<'stop' | 'delete' | null>(null)
const confirmMessage = ref('')
const pendingIds = ref<number[]>([])

// 全选状态
const isAllSelected = computed(() => {
  const selectableTasks = props.tasks.filter(t => t.status !== 'running' || confirmAction.value === 'stop')
  return selectableTasks.length > 0 && selectableTasks.every(t => selectedIds.value.has(t.id))
})

const isIndeterminate = computed(() => {
  const selectableTasks = props.tasks.filter(t => t.status !== 'running' || confirmAction.value === 'stop')
  const selectedCount = selectableTasks.filter(t => selectedIds.value.has(t.id)).length
  return selectedCount > 0 && selectedCount < selectableTasks.length
})

// 获取可操作的任务
function getSelectableTasks(action: 'stop' | 'delete'): Task[] {
  if (action === 'stop') {
    return props.tasks.filter(t => t.status === 'running' || t.status === 'paused')
  } else {
    return props.tasks.filter(t => t.status !== 'running')
  }
}

// 获取不符合条件的任务
function getInvalidTasks(action: 'stop' | 'delete', ids: number[]): Task[] {
  const selectable = getSelectableTasks(action)
  const selectableIds = new Set(selectable.map(t => t.id))
  return props.tasks.filter(t => ids.includes(t.id) && !selectableIds.has(t.id))
}

// 切换单个选择
function toggleSelect(id: number) {
  if (selectedIds.value.has(id)) {
    selectedIds.value.delete(id)
  } else {
    selectedIds.value.add(id)
  }
}

// 切换全选
function toggleSelectAll() {
  const selectableTasks = props.tasks
  if (isAllSelected.value) {
    selectedIds.value.clear()
  } else {
    selectableTasks.forEach(t => selectedIds.value.add(t.id))
  }
}

// 清空选择
function clearSelection() {
  selectedIds.value.clear()
}

// 执行批量操作
function handleBatchAction(action: 'stop' | 'delete') {
  const ids = Array.from(selectedIds.value)
  if (ids.length === 0) return

  const invalidTasks = getInvalidTasks(action, ids)
  pendingIds.value = ids

  if (invalidTasks.length > 0) {
    const invalidNames = invalidTasks.map(t => `#${t.id} (${t.status})`).join(', ')
    confirmMessage.value = `以下任务无法${action === 'stop' ? '停止' : '删除'}: ${invalidNames}\n\n是否继续操作其他 ${ids.length - invalidTasks.length} 个任务?`
  } else {
    const actionText = action === 'stop' ? '停止' : '删除'
    confirmMessage.value = `确定要${actionText}选中的 ${ids.length} 个任务吗?`
  }

  confirmAction.value = action
  showConfirmModal.value = true
}

// 确认批量操作
async function confirmBatchAction() {
  if (!confirmAction.value) return

  const action = confirmAction.value
  const ids = pendingIds.value.filter(id => {
    const task = props.tasks.find(t => t.id === id)
    if (!task) return false
    if (action === 'stop') {
      return task.status === 'running' || task.status === 'paused'
    } else {
      return task.status !== 'running'
    }
  })

  showConfirmModal.value = false

  if (ids.length > 0) {
    if (action === 'stop') {
      emit('batch-stop', ids)
    } else {
      emit('batch-delete', ids)
    }
  }

  clearSelection()
  confirmAction.value = null
  pendingIds.value = []
}

// 取消确认
function cancelConfirm() {
  showConfirmModal.value = false
  confirmAction.value = null
  pendingIds.value = []
}

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

// 暴露方法供父组件调用
defineExpose({
  clearSelection,
  selectedIds
})
</script>

<template>
  <div class="space-y-4">
    <!-- 批量操作工具栏 -->
    <div v-if="selectedIds.size > 0" class="flex items-center gap-4 p-3 bg-terminal-bg border border-terminal-border rounded">
      <span class="text-terminal-accent">已选择 {{ selectedIds.size }} 个任务</span>
      <div class="flex gap-2">
        <button
          @click="handleBatchAction('stop')"
          class="btn btn-danger text-xs px-3 py-1"
        >
          批量停止
        </button>
        <button
          @click="handleBatchAction('delete')"
          class="btn btn-danger text-xs px-3 py-1"
        >
          批量删除
        </button>
        <button
          @click="clearSelection"
          class="btn btn-secondary text-xs px-3 py-1"
        >
          取消选择
        </button>
      </div>
    </div>

    <!-- 确认弹窗 -->
    <div v-if="showConfirmModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div class="bg-terminal-bg border border-terminal-border rounded-lg p-6 max-w-md w-full mx-4">
        <h3 class="text-lg font-semibold mb-4">确认操作</h3>
        <p class="text-terminal-muted mb-6 whitespace-pre-wrap">{{ confirmMessage }}</p>
        <div class="flex justify-end gap-3">
          <button @click="cancelConfirm" class="btn btn-secondary">取消</button>
          <button @click="confirmBatchAction" class="btn btn-danger">确认</button>
        </div>
      </div>
    </div>

    <div class="card overflow-hidden">
      <table class="table-terminal" v-if="tasks.length > 0">
        <thead>
          <tr>
            <th class="w-10">
              <input
                type="checkbox"
                :checked="isAllSelected"
                :indeterminate="isIndeterminate"
                @change="toggleSelectAll"
                class="w-4 h-4 rounded border-terminal-border bg-terminal-bg accent-terminal-accent cursor-pointer"
              />
            </th>
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
            :class="{ 'bg-terminal-accent/10': selectedIds.has(task.id) }"
            class="cursor-pointer hover:bg-terminal-border/20"
          >
            <td @click.stop="toggleSelect(task.id)">
              <input
                type="checkbox"
                :checked="selectedIds.has(task.id)"
                @click.stop
                @change="toggleSelect(task.id)"
                class="w-4 h-4 rounded border-terminal-border bg-terminal-bg accent-terminal-accent cursor-pointer"
              />
            </td>
            <td @click="goToDetail(task.id)">
              <router-link :to="`/tasks/${task.id}`" class="text-terminal-accent hover:underline" @click.stop>
                #{{ task.id }}
              </router-link>
            </td>
            <td @click="goToDetail(task.id)">
              <router-link :to="`/tasks/${task.id}`" class="hover:text-terminal-accent" @click.stop>
                {{ task.name }}
              </router-link>
            </td>
            <td @click="goToDetail(task.id)">
              <span :class="['badge', getStatusClass(task.status)]">{{ task.status }}</span>
            </td>
            <td @click="goToDetail(task.id)" class="text-terminal-muted">{{ task.executor_type }}</td>
            <td @click="goToDetail(task.id)">{{ task.current_iteration.toLocaleString() }}</td>
            <td @click="goToDetail(task.id)" class="text-terminal-error">{{ task.total_crashes }}</td>
            <td @click="goToDetail(task.id)">{{ task.exec_per_sec.toFixed(1) }}/s</td>
            <td @click="goToDetail(task.id)" class="text-terminal-muted text-xs">{{ formatDate(task.created_at) }}</td>
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
  </div>
</template>
