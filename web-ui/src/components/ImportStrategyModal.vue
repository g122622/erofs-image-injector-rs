<template>
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg w-full max-w-xl mx-4 overflow-hidden">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-200 flex justify-between items-center">
        <h2 class="text-xl font-semibold text-gray-900">Import Strategy</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="p-6">
        <!-- Tabs -->
        <div class="flex border-b border-gray-200 mb-4">
          <button
            @click="activeTab = 'file'"
            :class="[
              'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
              activeTab === 'file'
                ? 'border-indigo-500 text-indigo-600'
                : 'border-transparent text-gray-500 hover:text-gray-700'
            ]"
          >
            Upload File
          </button>
          <button
            @click="activeTab = 'paste'"
            :class="[
              'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
              activeTab === 'paste'
                ? 'border-indigo-500 text-indigo-600'
                : 'border-transparent text-gray-500 hover:text-gray-700'
            ]"
          >
            Paste TOML
          </button>
        </div>

        <!-- File Upload Tab -->
        <div v-if="activeTab === 'file'" class="space-y-4">
          <div
            @dragover.prevent="dragOver = true"
            @dragleave="dragOver = false"
            @drop.prevent="handleDrop"
            :class="[
              'border-2 border-dashed rounded-lg p-8 text-center transition-colors',
              dragOver ? 'border-indigo-500 bg-indigo-50' : 'border-gray-300'
            ]"
          >
            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
            </svg>
            <p class="mt-2 text-sm text-gray-600">
              Drag and drop a TOML file here, or
              <label class="text-indigo-600 hover:text-indigo-500 cursor-pointer">
                browse
                <input
                  type="file"
                  accept=".toml"
                  @change="handleFileSelect"
                  class="hidden"
                />
              </label>
            </p>
            <p v-if="selectedFile" class="mt-2 text-sm text-gray-900">
              Selected: {{ selectedFile.name }}
            </p>
          </div>
        </div>

        <!-- Paste Tab -->
        <div v-if="activeTab === 'paste'" class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">TOML Content</label>
            <textarea
              v-model="tomlContent"
              rows="12"
              class="w-full px-3 py-2 border border-gray-300 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
              placeholder="Paste TOML strategy configuration here..."
            ></textarea>
          </div>
        </div>

        <!-- Error -->
        <div v-if="error" class="mt-4 p-3 bg-red-50 border border-red-200 rounded-md">
          <p class="text-sm text-red-600">{{ error }}</p>
        </div>

        <!-- Preview -->
        <div v-if="previewName" class="mt-4 p-3 bg-green-50 border border-green-200 rounded-md">
          <p class="text-sm text-green-700">
            <span class="font-medium">Strategy:</span> {{ previewName }}
          </p>
          <p v-if="previewDescription" class="text-sm text-green-600 mt-1">
            {{ previewDescription }}
          </p>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-gray-200 flex justify-end gap-3">
        <button
          @click="$emit('close')"
          class="px-4 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
        >
          Cancel
        </button>
        <button
          @click="handleImport"
          :disabled="!canImport"
          :class="[
            'px-4 py-2 rounded-md text-sm font-medium',
            canImport
              ? 'bg-indigo-600 text-white hover:bg-indigo-700'
              : 'bg-gray-300 text-gray-500 cursor-not-allowed'
          ]"
        >
          Import
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'import', content: string): void
}>()

const activeTab = ref<'file' | 'paste'>('file')
const dragOver = ref(false)
const selectedFile = ref<File | null>(null)
const tomlContent = ref('')
const error = ref('')
const previewName = ref('')
const previewDescription = ref('')

const canImport = computed(() => {
  if (activeTab.value === 'file') {
    return selectedFile.value !== null
  } else {
    return tomlContent.value.trim().length > 0
  }
})

function handleDrop(e: DragEvent) {
  dragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    selectFile(files[0])
  }
}

function handleFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  const files = target.files
  if (files && files.length > 0) {
    selectFile(files[0])
  }
}

function selectFile(file: File) {
  if (!file.name.endsWith('.toml')) {
    error.value = 'Please select a TOML file'
    return
  }

  selectedFile.value = file
  error.value = ''
  previewName.value = ''
  previewDescription.value = ''

  const reader = new FileReader()
  reader.onload = () => {
    const content = reader.result as string
    tomlContent.value = content
    parsePreview(content)
  }
  reader.onerror = () => {
    error.value = 'Failed to read file'
  }
  reader.readAsText(file)
}

function parsePreview(content: string) {
  try {
    // Simple TOML parsing for preview
    const nameMatch = content.match(/^name\s*=\s*"([^"]+)"/m)
    const descMatch = content.match(/^description\s*=\s*"([^"]+)"/m)

    if (nameMatch) {
      previewName.value = nameMatch[1]
    }
    if (descMatch) {
      previewDescription.value = descMatch[1]
    }
  } catch {
    // Ignore parsing errors for preview
  }
}

async function handleImport() {
  error.value = ''

  if (activeTab.value === 'file' && selectedFile.value) {
    try {
      const content = await selectedFile.value.text()
      emit('import', content)
    } catch {
      error.value = 'Failed to read file'
    }
  } else if (activeTab.value === 'paste' && tomlContent.value.trim()) {
    emit('import', tomlContent.value.trim())
  }
}

// Watch for TOML content changes
watch(tomlContent, (value) => {
  if (value.trim()) {
    parsePreview(value.trim())
  } else {
    previewName.value = ''
    previewDescription.value = ''
  }
})
</script>
