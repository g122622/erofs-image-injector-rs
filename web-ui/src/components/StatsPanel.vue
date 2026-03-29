<script setup lang="ts">
import { useTaskStore } from '@/stores/task'

const taskStore = useTaskStore()
</script>

<template>
  <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
    <div class="card p-4">
      <div class="text-terminal-muted text-sm">Total Tasks</div>
      <div class="text-2xl font-bold text-terminal-text">{{ taskStore.stats?.total ?? 0 }}</div>
    </div>
    <div class="card p-4">
      <div class="text-terminal-muted text-sm">Running</div>
      <div class="text-2xl font-bold text-terminal-accent">{{ taskStore.stats?.running ?? 0 }}</div>
    </div>
    <div class="card p-4">
      <div class="text-terminal-muted text-sm">Completed</div>
      <div class="text-2xl font-bold text-terminal-success">{{ taskStore.stats?.completed ?? 0 }}</div>
    </div>
    <div class="card p-4">
      <div class="text-terminal-muted text-sm">Total Crashes</div>
      <div class="text-2xl font-bold text-terminal-error">{{ taskStore.stats?.total_crashes ?? 0 }}</div>
    </div>
  </div>
</template>
