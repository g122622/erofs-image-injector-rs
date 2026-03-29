<template>
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-terminal-bg border border-terminal-border rounded-lg w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div class="px-4 py-3 border-b border-terminal-border flex justify-between items-center">
        <div>
          <h2 class="text-lg font-semibold text-terminal-fg">Kernel Log</h2>
          <p class="text-sm text-terminal-muted">{{ crashId ? `Crash #${crashId}` : '' }}</p>
        </div>
        <button @click="$emit('close')" class="text-terminal-muted hover:text-terminal-fg">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-auto p-4">
        <div v-if="loading" class="flex items-center justify-center h-32">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-terminal-accent"></div>
        </div>
        <div v-else-if="error" class="text-terminal-error p-4">
          {{ error }}
        </div>
        <pre v-else class="text-sm font-mono text-terminal-fg whitespace-pre-wrap break-all bg-terminal-surface p-4 rounded border border-terminal-border overflow-auto max-h-[60vh]">{{ logContent }}</pre>
      </div>

      <!-- Footer -->
      <div class="px-4 py-3 border-t border-terminal-border flex justify-end gap-3">
        <button
          @click="copyToClipboard"
          class="btn btn-secondary flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
          </svg>
          {{ copied ? 'Copied!' : 'Copy' }}
        </button>
        <button
          @click="$emit('close')"
          class="btn btn-primary"
        >
          Close
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  crashId: number
}>()

const loading = ref(true)
const error = ref('')
const logContent = ref('')
const copied = ref(false)

onMounted(async () => {
  try {
    loading.value = true
    const response = await fetch(`/api/crashes/${props.crashId}/log`)
    if (!response.ok) {
      if (response.status === 404) {
        error.value = 'No kernel log available for this crash'
      } else {
        error.value = `Failed to load log: ${response.statusText}`
      }
      return
    }
    logContent.value = await response.text()
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load log'
  } finally {
    loading.value = false
  }
})

async function copyToClipboard() {
  try {
    await navigator.clipboard.writeText(logContent.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    const textArea = document.createElement('textarea')
    textArea.value = logContent.value
    document.body.appendChild(textArea)
    textArea.select()
    document.execCommand('copy')
    document.body.removeChild(textArea)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  }
}
</script>

<style scoped>
.btn {
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.15s;
}

.btn-primary {
  background-color: #00d9ff;
  color: white;
}

.btn-primary:hover {
  background-color: rgba(0, 217, 255, 0.9);
}

.btn-secondary {
  background-color: #12121a;
  color: #cdd6f4;
  border: 1px solid #1e1e2e;
}

.btn-secondary:hover {
  background-color: rgba(30, 30, 46, 0.5);
}
</style>
