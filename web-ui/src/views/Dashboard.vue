<script setup lang="ts">
import { useTaskStore } from '@/stores/task'
import StatsPanel from '@/components/StatsPanel.vue'
import TaskList from '@/components/TaskList.vue'
import CreateTaskForm from '@/components/CreateTaskForm.vue'

const taskStore = useTaskStore()
</script>

<template>
  <div class="space-y-6">
    <StatsPanel />

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-2">
        <h2 class="text-lg font-semibold mb-4">Active Tasks</h2>
        <TaskList :tasks="taskStore.tasks.filter(t => t.status === 'running' || t.status === 'pending')" />
      </div>

      <div>
        <h2 class="text-lg font-semibold mb-4">Create Task</h2>
        <CreateTaskForm />
      </div>
    </div>
  </div>
</template>
