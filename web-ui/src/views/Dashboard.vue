<script setup lang="ts">
import { ref } from 'vue'
import { useTaskStore } from '@/stores/task'
import StatsPanel from '@/components/StatsPanel.vue'
import TaskList from '@/components/TaskList.vue'
import CreateTaskForm from '@/components/CreateTaskForm.vue'

const taskStore = useTaskStore()
const taskListRef = ref<InstanceType<typeof TaskList> | null>(null)

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
    <StatsPanel />

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-2">
        <h2 class="text-lg font-semibold mb-4">Active Tasks</h2>
        <TaskList
          ref="taskListRef"
          :tasks="taskStore.tasks.filter(t => t.status === 'running' || t.status === 'pending')"
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
