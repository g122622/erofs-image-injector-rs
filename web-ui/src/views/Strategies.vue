<template>
  <div class="min-h-screen bg-gray-100">
    <!-- Header -->
    <header class="bg-white shadow">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
        <div class="flex justify-between items-center">
          <div>
            <h1 class="text-2xl font-bold text-gray-900">Mutation Strategies</h1>
            <p class="text-sm text-gray-500">Configure and manage mutation strategy templates</p>
          </div>
          <div class="flex gap-3">
            <button
              @click="showImportModal = true"
              class="px-4 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
            >
              Import
            </button>
            <button
              @click="createNewStrategy"
              class="px-4 py-2 bg-indigo-600 text-white rounded-md text-sm font-medium hover:bg-indigo-700"
            >
              Create Strategy
            </button>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <!-- Loading State -->
      <div v-if="loading" class="flex justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600"></div>
      </div>

      <!-- Strategy Grid -->
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <!-- Strategy Card -->
        <div
          v-for="strategy in strategies"
          :key="strategy.id"
          class="bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden hover:shadow-md transition-shadow"
        >
          <div class="p-5">
            <!-- Header -->
            <div class="flex justify-between items-start mb-3">
              <div>
                <h3 class="text-lg font-semibold text-gray-900">{{ strategy.name }}</h3>
                <span
                  v-if="strategy.is_builtin"
                  class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800"
                >
                  Built-in
                </span>
              </div>
              <div class="flex gap-1">
                <button
                  @click="editStrategy(strategy)"
                  class="p-1.5 text-gray-400 hover:text-gray-600 rounded"
                  title="Edit"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                  </svg>
                </button>
                <button
                  @click="duplicateStrategy(strategy)"
                  class="p-1.5 text-gray-400 hover:text-gray-600 rounded"
                  title="Duplicate"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </button>
                <button
                  @click="exportStrategy(strategy)"
                  class="p-1.5 text-gray-400 hover:text-gray-600 rounded"
                  title="Export"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                  </svg>
                </button>
                <button
                  v-if="!strategy.is_builtin"
                  @click="confirmDelete(strategy)"
                  class="p-1.5 text-gray-400 hover:text-red-600 rounded"
                  title="Delete"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>

            <!-- Description -->
            <p class="text-sm text-gray-600 mb-4">{{ strategy.description || 'No description' }}</p>

            <!-- Mutator Tags -->
            <div class="flex flex-wrap gap-1.5 mb-4">
              <span
                v-for="(_config, mutator) in getEnabledMutators(strategy)"
                :key="mutator"
                class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800"
              >
                {{ getMutatorName(mutator as MutatorType) }}
              </span>
            </div>

            <!-- Stats -->
            <div class="flex justify-between text-xs text-gray-500">
              <span>{{ getEnabledMutatorCount(strategy) }} mutators enabled</span>
              <span v-if="strategy.adaptive_enabled" class="text-indigo-600">Adaptive</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Empty State -->
      <div v-if="!loading && strategies.length === 0" class="text-center py-12">
        <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
        </svg>
        <h3 class="mt-2 text-sm font-medium text-gray-900">No strategies</h3>
        <p class="mt-1 text-sm text-gray-500">Get started by creating a new mutation strategy.</p>
        <div class="mt-6">
          <button
            @click="createNewStrategy"
            class="inline-flex items-center px-4 py-2 bg-indigo-600 text-white rounded-md text-sm font-medium hover:bg-indigo-700"
          >
            Create Strategy
          </button>
        </div>
      </div>
    </main>

    <!-- Edit/Create Modal -->
    <StrategyEditorModal
      v-if="showEditModal"
      :strategy="editingStrategy"
      @close="closeEditModal"
      @save="saveStrategy"
    />

    <!-- Import Modal -->
    <ImportStrategyModal
      v-if="showImportModal"
      @close="showImportModal = false"
      @import="handleImport"
    />

    <!-- Delete Confirmation Modal -->
    <div v-if="deletingStrategy" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
        <h3 class="text-lg font-medium text-gray-900 mb-4">Delete Strategy</h3>
        <p class="text-sm text-gray-500 mb-6">
          Are you sure you want to delete "{{ deletingStrategy.name }}"? This action cannot be undone.
        </p>
        <div class="flex justify-end gap-3">
          <button
            @click="deletingStrategy = null"
            class="px-4 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
          >
            Cancel
          </button>
          <button
            @click="deleteStrategy"
            class="px-4 py-2 bg-red-600 text-white rounded-md text-sm font-medium hover:bg-red-700"
          >
            Delete
          </button>
        </div>
      </div>
    </div>

    <!-- Notification -->
    <div
      v-if="notification"
      class="fixed bottom-4 right-4 z-50 px-4 py-3 rounded-md shadow-lg"
      :class="notification.type === 'success' ? 'bg-green-600 text-white' : 'bg-red-600 text-white'"
    >
      {{ notification.message }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api'
import type { StrategyTemplate, MutatorType } from '@/types'
import { MUTATOR_INFO } from '@/types'
import StrategyEditorModal from '@/components/StrategyEditorModal.vue'
import ImportStrategyModal from '@/components/ImportStrategyModal.vue'

const loading = ref(true)
const strategies = ref<StrategyTemplate[]>([])
const showEditModal = ref(false)
const showImportModal = ref(false)
const editingStrategy = ref<StrategyTemplate | null>(null)
const deletingStrategy = ref<StrategyTemplate | null>(null)
const notification = ref<{ message: string; type: 'success' | 'error' } | null>(null)

function showNotify(message: string, type: 'success' | 'error') {
  notification.value = { message, type }
  setTimeout(() => {
    notification.value = null
  }, 3000)
}

onMounted(async () => {
  await loadStrategies()
})

async function loadStrategies() {
  try {
    loading.value = true
    strategies.value = await api.listStrategies()
  } catch (error) {
    showNotify(`Failed to load strategies: ${error}`, 'error')
  } finally {
    loading.value = false
  }
}

function getEnabledMutators(strategy: StrategyTemplate): Record<string, unknown> {
  const enabled: Record<string, unknown> = {}
  for (const [key, config] of Object.entries(strategy.mutators)) {
    if ((config as { enabled: boolean }).enabled) {
      enabled[key] = config
    }
  }
  return enabled
}

function getEnabledMutatorCount(strategy: StrategyTemplate): number {
  return Object.values(strategy.mutators).filter((c) => c.enabled).length
}

function getMutatorName(mutator: MutatorType): string {
  return MUTATOR_INFO[mutator]?.name || mutator
}

function createNewStrategy() {
  editingStrategy.value = null
  showEditModal.value = true
}

function editStrategy(strategy: StrategyTemplate) {
  editingStrategy.value = { ...strategy, mutators: JSON.parse(JSON.stringify(strategy.mutators)) }
  showEditModal.value = true
}

function closeEditModal() {
  showEditModal.value = false
  editingStrategy.value = null
}

async function saveStrategy(strategy: StrategyTemplate) {
  try {
    if (strategy.id && !strategy.is_builtin) {
      await api.updateStrategy(strategy.id, {
        name: strategy.name,
        description: strategy.description,
        mutators: strategy.mutators,
        layers: strategy.layers,
        adaptive_rules: strategy.adaptive_rules,
        adaptive_enabled: strategy.adaptive_enabled,
      })
      showNotify( 'Strategy updated successfully', 'success')
    } else {
      const created = await api.createStrategy({
        name: strategy.name,
        description: strategy.description,
        mutators: strategy.mutators,
        layers: strategy.layers,
        adaptive_rules: strategy.adaptive_rules,
        adaptive_enabled: strategy.adaptive_enabled,
      })
      strategies.value.push(created)
      showNotify( 'Strategy created successfully', 'success')
    }
    await loadStrategies()
    closeEditModal()
  } catch (error) {
    showNotify( `Failed to save strategy: ${error}`, 'error')
  }
}

async function duplicateStrategy(strategy: StrategyTemplate) {
  try {
    const newName = `${strategy.name} (Copy)`
    const created = await api.duplicateStrategy(strategy.id!, newName)
    strategies.value.push(created)
    showNotify( 'Strategy duplicated successfully', 'success')
  } catch (error) {
    showNotify( `Failed to duplicate strategy: ${error}`, 'error')
  }
}

async function exportStrategy(strategy: StrategyTemplate) {
  try {
    const result = await api.exportStrategy(strategy.id!)
    const blob = new Blob([result.content], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${strategy.name.replace(/\s+/g, '_')}.toml`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    showNotify( 'Strategy exported successfully', 'success')
  } catch (error) {
    showNotify( `Failed to export strategy: ${error}`, 'error')
  }
}

function confirmDelete(strategy: StrategyTemplate) {
  deletingStrategy.value = strategy
}

async function deleteStrategy() {
  if (!deletingStrategy.value) return

  try {
    await api.deleteStrategy(deletingStrategy.value.id!)
    strategies.value = strategies.value.filter((s) => s.id !== deletingStrategy.value!.id)
    showNotify( 'Strategy deleted successfully', 'success')
  } catch (error) {
    showNotify( `Failed to delete strategy: ${error}`, 'error')
  } finally {
    deletingStrategy.value = null
  }
}

async function handleImport(content: string) {
  try {
    const imported = await api.importStrategy(content)
    strategies.value.push(imported)
    showImportModal.value = false
    showNotify( 'Strategy imported successfully', 'success')
  } catch (error) {
    showNotify( `Failed to import strategy: ${error}`, 'error')
  }
}
</script>
