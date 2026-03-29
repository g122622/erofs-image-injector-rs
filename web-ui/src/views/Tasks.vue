<script setup lang="ts">
import { onMounted } from 'vue'
import { useTaskStore } from '@/stores/task'
import TaskList from '@/components/TaskList.vue'
import CreateTaskForm from '@/components/CreateTaskForm.vue'

const taskStore = useTaskStore()

onMounted(() => {
  taskStore.fetchTasks()
})
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
        <TaskList :tasks="taskStore.tasks" />
      </div>

      <div>
        <h2 class="text-lg font-semibold mb-4">Create Task</h2>
        <CreateTaskForm />
      </div>
    </div>
  </div>
</template>
