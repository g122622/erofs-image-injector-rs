<template>
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-terminal-bg border border-terminal-border rounded-lg w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div class="px-4 py-3 border-b border-terminal-border flex justify-between items-center">
        <div>
          <h2 class="text-lg font-semibold text-terminal-fg">Reproduction Script</h2>
          <p class="text-sm text-terminal-muted">{{ description }}</p>
        </div>
        <button @click="$emit('close')" class="text-terminal-muted hover:text-terminal-fg">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Editor -->
      <div class="flex-1 min-h-[400px]">
        <div v-if="loading" class="flex items-center justify-center h-full">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-terminal-accent"></div>
        </div>
        <div v-else-if="error" class="flex items-center justify-center h-full">
          <div class="text-terminal-error">{{ error }}</div>
        </div>
        <VueMonacoEditor
          v-else
          v-model:value="script"
          language="shell"
          theme="vs-dark"
          :options="editorOptions"
          class="h-full"
          style="height: 500px;"
        />
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
          @click="downloadScript"
          class="btn btn-primary flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
          Download
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'
import { api } from '@/api'

const props = defineProps<{
  crashId: number
}>()

const loading = ref(true)
const error = ref('')
const script = ref('')
const description = ref('')
const copied = ref(false)

const editorOptions = {
  readOnly: true,
  minimap: { enabled: false },
  fontSize: 13,
  lineNumbers: 'on' as const,
  scrollBeyondLastLine: false,
  automaticLayout: true,
  padding: { top: 10, bottom: 10 },
  wordWrap: 'on' as const,
  folding: true,
  renderWhitespace: 'selection' as const,
}

onMounted(async () => {
  try {
    loading.value = true
    const result = await api.getCrashRepro(props.crashId)
    script.value = result.script
    description.value = result.description
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load reproduction script'
  } finally {
    loading.value = false
  }
})

async function copyToClipboard() {
  try {
    await navigator.clipboard.writeText(script.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    // Fallback for older browsers
    const textArea = document.createElement('textarea')
    textArea.value = script.value
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

function downloadScript() {
  const blob = new Blob([script.value], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `repro-crash-${props.crashId}.sh`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
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
