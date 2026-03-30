<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useTaskStore } from '@/stores/task'
import TaskList from '@/components/TaskList.vue'
import CreateTaskForm from '@/components/CreateTaskForm.vue'

const taskStore = useTaskStore()
const taskListRef = ref<InstanceType<typeof TaskList> | null>(null)

onMounted(() => {
  taskStore.fetchTasks()
})

async function handleBatchStop(ids: number[]) {
  try {
    const result = await taskStore.batchStopTasks(ids)
    if (result.failed.length > 0) {
      const failedInfo = result.failed.map(f => `#${f.id}: ${f.error}`).join('\n')
      alert(`部分任务停止失败:\n${failedInfo}`)
    }
  } catch (e) {
    console.error('Failed to batch stop tasks:', e)
    alert('批量停止失败')
  }
}

async function handleBatchDelete(ids: number[]) {
  try {
    const result = await taskStore.batchDeleteTasks(ids)
    if (result.failed.length > 0) {
      const failedInfo = result.failed.map(f => `#${f.id}: ${f.error}`).join('\n')
      alert(`部分任务删除失败:\n${failedInfo}`)
    }
  } catch (e) {
    console.error('Failed to batch delete tasks:', e)
    alert('批量删除失败')
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex justify-between items-center">
      <h1 class="text-2xl font-bold">Tasks</h1>
      <button
        @click="taskStore.fetchTasks()"
        class="btn btn-secondary"
      >
        Refresh
      </button>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-2">
        <h2 class="text-lg font-semibold mb-4">All Tasks</h2>
        <TaskList
          ref="taskListRef"
          :tasks="taskStore.tasks"
          @batch-stop="handleBatchStop"
          @batch-delete="handleBatchDelete"
        />
      </div>

      <div>
        <h2 class="text-lg font-semibold mb-4">Create Task</h2>
        <CreateTaskForm />
      </div>
    </div>
  </div>
</template>
