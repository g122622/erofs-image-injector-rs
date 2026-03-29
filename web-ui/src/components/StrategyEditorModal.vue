<template>
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 overflow-y-auto">
    <div class="bg-white rounded-lg w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col my-4">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-200 flex justify-between items-center">
        <h2 class="text-xl font-semibold text-gray-900">
          {{ isNew ? 'Create Strategy' : 'Edit Strategy' }}
        </h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-600">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        <!-- Basic Info -->
        <div class="mb-6">
          <h3 class="text-sm font-medium text-gray-900 mb-3">Basic Information</h3>
          <div class="grid grid-cols-1 gap-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
              <input
                v-model="form.name"
                type="text"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500"
                placeholder="Strategy name"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
              <textarea
                v-model="form.description"
                rows="2"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500"
                placeholder="Strategy description"
              ></textarea>
            </div>
          </div>
        </div>

        <!-- Mode Toggle -->
        <div class="mb-6">
          <div class="flex items-center justify-between">
            <h3 class="text-sm font-medium text-gray-900">Mutator Configuration</h3>
            <div class="flex items-center gap-2">
              <span class="text-sm text-gray-500">Simple</span>
              <button
                @click="advancedMode = !advancedMode"
                :class="[
                  'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
                  advancedMode ? 'bg-indigo-600' : 'bg-gray-200'
                ]"
              >
                <span
                  :class="[
                    'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                    advancedMode ? 'translate-x-6' : 'translate-x-1'
                  ]"
                />
              </button>
              <span class="text-sm text-gray-500">Advanced</span>
            </div>
          </div>
        </div>

        <!-- Simple Mode -->
        <div v-if="!advancedMode" class="mb-6">
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <div
              v-for="(info, mutator) in MUTATOR_INFO"
              :key="mutator"
              class="border rounded-lg p-4"
              :class="form.mutators[mutator]?.enabled ? 'border-indigo-500 bg-indigo-50' : 'border-gray-200'"
            >
              <div class="flex items-start justify-between mb-2">
                <div class="flex items-center">
                  <input
                    type="checkbox"
                    :checked="form.mutators[mutator]?.enabled"
                    @change="toggleMutator(mutator as MutatorType)"
                    class="h-4 w-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
                  />
                  <label class="ml-2 text-sm font-medium text-gray-900">{{ info.name }}</label>
                </div>
                <span
                  :class="[
                    'px-2 py-0.5 text-xs rounded',
                    info.category === 'basic' ? 'bg-gray-100 text-gray-600' :
                    info.category === 'structure' ? 'bg-blue-100 text-blue-600' :
                    'bg-purple-100 text-purple-600'
                  ]"
                >
                  {{ info.category }}
                </span>
              </div>
              <p class="text-xs text-gray-500 mb-2">{{ info.description }}</p>
              <div v-if="form.mutators[mutator]?.enabled" class="mt-2">
                <label class="block text-xs text-gray-500 mb-1">Weight: {{ form.mutators[mutator]?.weight }}</label>
                <input
                  type="range"
                  v-model.number="form.mutators[mutator].weight"
                  min="0"
                  max="1000"
                  step="10"
                  class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Advanced Mode -->
        <div v-else class="mb-6 space-y-6">
          <!-- Categories -->
          <div v-for="category in ['basic', 'structure', 'targeted']" :key="category">
            <h4 class="text-sm font-medium text-gray-700 mb-3 capitalize">{{ category }} Mutators</h4>
            <div class="space-y-3">
              <div
                v-for="(info, mutatorKey) in getMutatorsByCategory(category)"
                :key="mutatorKey"
                class="border rounded-lg p-4"
                :class="getMutatorEnabled(mutatorKey as MutatorType) ? 'border-indigo-500' : 'border-gray-200'"
              >
                <div class="flex items-center justify-between mb-3">
                  <div class="flex items-center">
                    <input
                      type="checkbox"
                      :checked="getMutatorEnabled(mutatorKey as MutatorType)"
                      @change="toggleMutator(mutatorKey as MutatorType)"
                      class="h-4 w-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
                    />
                    <label class="ml-2 text-sm font-medium text-gray-900">{{ info.name }}</label>
                  </div>
                  <span class="text-xs text-gray-500">{{ info.description }}</span>
                </div>

                <div v-if="getMutatorEnabled(mutatorKey as MutatorType)" class="grid grid-cols-2 gap-4 pl-6">
                  <div>
                    <label class="block text-xs text-gray-500 mb-1">Weight (0-1000)</label>
                    <input
                      type="number"
                      :value="getMutatorWeight(mutatorKey as MutatorType)"
                      @input="setMutatorWeight(mutatorKey as MutatorType, ($event.target as HTMLInputElement).valueAsNumber)"
                      min="0"
                      max="1000"
                      class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    />
                  </div>
                  <div>
                    <label class="block text-xs text-gray-500 mb-1">Min Iterations</label>
                    <input
                      type="number"
                      :value="getMutatorMinIterations(mutatorKey as MutatorType)"
                      @input="setMutatorMinIterations(mutatorKey as MutatorType, ($event.target as HTMLInputElement).valueAsNumber)"
                      min="0"
                      placeholder="Optional"
                      class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    />
                  </div>
                  <div>
                    <label class="block text-xs text-gray-500 mb-1">Max Iterations</label>
                    <input
                      type="number"
                      :value="getMutatorMaxIterations(mutatorKey as MutatorType)"
                      @input="setMutatorMaxIterations(mutatorKey as MutatorType, ($event.target as HTMLInputElement).valueAsNumber)"
                      min="0"
                      placeholder="Optional"
                      class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Adaptive Weights -->
          <div class="border-t pt-6">
            <div class="flex items-center justify-between mb-4">
              <h4 class="text-sm font-medium text-gray-700">Adaptive Weights</h4>
              <label class="flex items-center">
                <input
                  type="checkbox"
                  v-model="form.adaptive_enabled"
                  class="h-4 w-4 text-indigo-600 border-gray-300 rounded focus:ring-indigo-500"
                />
                <span class="ml-2 text-sm text-gray-600">Enable adaptive weight adjustment</span>
              </label>
            </div>
            <p class="text-xs text-gray-500 mb-4">
              When enabled, weights will be automatically adjusted based on crash discovery rates.
            </p>
          </div>
        </div>

        <!-- Validation Errors -->
        <div v-if="validationError" class="mb-4 p-3 bg-red-50 border border-red-200 rounded-md">
          <p class="text-sm text-red-600">{{ validationError }}</p>
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
          @click="handleSave"
          :disabled="!isValid"
          :class="[
            'px-4 py-2 rounded-md text-sm font-medium',
            isValid
              ? 'bg-indigo-600 text-white hover:bg-indigo-700'
              : 'bg-gray-300 text-gray-500 cursor-not-allowed'
          ]"
        >
          {{ isNew ? 'Create' : 'Save' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { StrategyTemplate, MutatorType, MutatorConfig } from '@/types'
import { MUTATOR_INFO } from '@/types'

const props = defineProps<{
  strategy: StrategyTemplate | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', strategy: StrategyTemplate): void
}>()

const isNew = computed(() => !props.strategy?.id)

const advancedMode = ref(false)
const validationError = ref('')

interface MutatorConfigInternal extends MutatorConfig {
  enabled: boolean
  weight: number
  min_iterations?: number
  max_iterations?: number
  params?: Record<string, unknown>
}

const form = ref<StrategyTemplate>({
  id: undefined,
  name: '',
  description: '',
  is_builtin: false,
  created_at: undefined,
  updated_at: undefined,
  mutators: {} as Record<MutatorType, MutatorConfigInternal>,
  layers: [],
  adaptive_rules: [],
  adaptive_enabled: false,
})

onMounted(() => {
  // Initialize all mutators
  const mutators: Record<MutatorType, MutatorConfigInternal> = {} as Record<MutatorType, MutatorConfigInternal>
  for (const mutator of Object.keys(MUTATOR_INFO) as MutatorType[]) {
    mutators[mutator] = {
      enabled: false,
      weight: 100,
      min_iterations: undefined,
      max_iterations: undefined,
      params: {},
    }
  }

  if (props.strategy) {
    form.value = {
      ...props.strategy,
      mutators: { ...mutators, ...props.strategy.mutators } as Record<MutatorType, MutatorConfigInternal>,
    }
  } else {
    form.value.mutators = mutators
  }
})

const isValid = computed(() => {
  if (!form.value.name.trim()) return false
  const enabledCount = Object.values(form.value.mutators).filter((m) => m.enabled).length
  if (enabledCount === 0) return false
  return true
})

function toggleMutator(mutator: MutatorType) {
  if (!form.value.mutators[mutator]) {
    form.value.mutators[mutator] = {
      enabled: true,
      weight: 100,
      min_iterations: undefined,
      max_iterations: undefined,
      params: {},
    }
  } else {
    form.value.mutators[mutator].enabled = !form.value.mutators[mutator].enabled
  }
}

function getMutatorEnabled(mutator: MutatorType): boolean {
  return form.value.mutators[mutator]?.enabled ?? false
}

function getMutatorWeight(mutator: MutatorType): number {
  return form.value.mutators[mutator]?.weight ?? 100
}

function setMutatorWeight(mutator: MutatorType, value: number) {
  if (form.value.mutators[mutator]) {
    form.value.mutators[mutator].weight = value
  }
}

function getMutatorMinIterations(mutator: MutatorType): number | undefined {
  return form.value.mutators[mutator]?.min_iterations
}

function setMutatorMinIterations(mutator: MutatorType, value: number | undefined) {
  if (form.value.mutators[mutator]) {
    form.value.mutators[mutator].min_iterations = value
  }
}

function getMutatorMaxIterations(mutator: MutatorType): number | undefined {
  return form.value.mutators[mutator]?.max_iterations
}

function setMutatorMaxIterations(mutator: MutatorType, value: number | undefined) {
  if (form.value.mutators[mutator]) {
    form.value.mutators[mutator].max_iterations = value
  }
}

function getMutatorsByCategory(category: string): Record<string, { name: string; description: string; category: string }> {
  const result: Record<string, { name: string; description: string; category: string }> = {}
  for (const [mutator, info] of Object.entries(MUTATOR_INFO)) {
    if (info.category === category) {
      result[mutator] = info
    }
  }
  return result
}

function handleSave() {
  validationError.value = ''

  if (!form.value.name.trim()) {
    validationError.value = 'Strategy name is required'
    return
  }

  const enabledCount = Object.values(form.value.mutators).filter((m) => m.enabled).length
  if (enabledCount === 0) {
    validationError.value = 'At least one mutator must be enabled'
    return
  }

  // Check for zero-weight enabled mutators
  for (const [mutator, config] of Object.entries(form.value.mutators)) {
    if (config.enabled && config.weight === 0) {
      validationError.value = `Weight for enabled mutator '${mutator}' cannot be zero`
      return
    }
  }

  emit('save', form.value)
}
</script>
