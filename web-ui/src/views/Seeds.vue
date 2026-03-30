<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useSeedStore } from '@/stores/seed'
import { useNotificationStore } from '@/stores/notification'
import type { Seed, SeedTemplate, SeedConfig, CreateSeedRequest } from '@/types/seed'
import { DEFAULT_SEED_CONFIG } from '@/types/seed'

const seedStore = useSeedStore()
const notificationStore = useNotificationStore()

// UI State
const selectedSeedId = ref<number | null>(null)
const showDeleteConfirm = ref(false)
const showGenerateModal = ref(false)
const showUploadModal = ref(false)
const activeTab = ref<'details' | 'generate' | 'templates'>('details')
const searchQuery = ref('')

// Generate form state
const generateForm = ref<{
  name: string
  count: number
  config: SeedConfig
  selectedTemplate: string | null
}>({
  name: '',
  count: 1,
  config: JSON.parse(JSON.stringify(DEFAULT_SEED_CONFIG)),
  selectedTemplate: null,
})

// Upload form state
const uploadForm = ref<{
  name: string
  file: File | null
}>({
  name: '',
  file: null,
})

// Computed
const selectedSeed = computed(() => {
  if (!selectedSeedId.value) return null
  return seedStore.seeds.find(s => s.id === selectedSeedId.value) || null
})

const filteredSeeds = computed(() => {
  if (!searchQuery.value) return seedStore.seeds
  const query = searchQuery.value.toLowerCase()
  return seedStore.seeds.filter(s =>
    s.name.toLowerCase().includes(query) ||
    s.tags?.toLowerCase().includes(query)
  )
})

// File input ref
const fileInput = ref<HTMLInputElement | null>(null)

// Methods
function selectSeed(seed: Seed) {
  selectedSeedId.value = seed.id
  activeTab.value = 'details'
}

function confirmDelete(seed: Seed) {
  selectedSeedId.value = seed.id
  showDeleteConfirm.value = true
}

async function deleteSeed() {
  if (!selectedSeedId.value) return
  try {
    await seedStore.deleteSeed(selectedSeedId.value)
    notificationStore.addNotification('seed_deleted', 'success', '删除成功', '种子文件已删除')
    selectedSeedId.value = null
    showDeleteConfirm.value = false
  } catch (e) {
    notificationStore.addNotification('seed_delete_failed', 'error', '删除失败', e instanceof Error ? e.message : '未知错误')
  }
}

async function downloadSeed(seed: Seed) {
  try {
    await seedStore.downloadSeed(seed.id)
    notificationStore.addNotification('seed_downloaded', 'success', '下载成功', `已下载 ${seed.name}.erofs`)
  } catch (e) {
    notificationStore.addNotification('seed_download_failed', 'error', '下载失败', e instanceof Error ? e.message : '未知错误')
  }
}

function openGenerateModal(template?: SeedTemplate) {
  if (template) {
    generateForm.value.config = JSON.parse(JSON.stringify(template.config))
    generateForm.value.selectedTemplate = template.id
  } else {
    generateForm.value.config = JSON.parse(JSON.stringify(DEFAULT_SEED_CONFIG))
    generateForm.value.selectedTemplate = null
  }
  generateForm.value.name = ''
  generateForm.value.count = 1
  showGenerateModal.value = true
}

function openUploadModal() {
  uploadForm.value.name = ''
  uploadForm.value.file = null
  showUploadModal.value = true
}

async function generateSeeds() {
  if (!generateForm.value.name) {
    notificationStore.addNotification('validation', 'error', '验证错误', '请输入种子名称')
    return
  }

  try {
    const request: CreateSeedRequest = {
      name: generateForm.value.name,
      config: generateForm.value.config,
      count: generateForm.value.count,
    }
    const seeds = await seedStore.generateSeeds(request)
    notificationStore.addNotification('seeds_generated', 'success', '生成成功', `已生成 ${seeds.length} 个种子文件`)
    showGenerateModal.value = false
  } catch (e) {
    notificationStore.addNotification('seed_generate_failed', 'error', '生成失败', e instanceof Error ? e.message : '未知错误')
  }
}

function handleFileSelect(event: Event) {
  const target = event.target as HTMLInputElement
  if (target.files && target.files[0]) {
    uploadForm.value.file = target.files[0]
    if (!uploadForm.value.name) {
      uploadForm.value.name = target.files[0].name.replace(/\.erofs$/i, '')
    }
  }
}

async function uploadSeed() {
  if (!uploadForm.value.name || !uploadForm.value.file) {
    notificationStore.addNotification('validation', 'error', '验证错误', '请填写名称并选择文件')
    return
  }

  try {
    await seedStore.uploadSeed(uploadForm.value.name, {}, uploadForm.value.file)
    notificationStore.addNotification('seed_uploaded', 'success', '上传成功', `已上传 ${uploadForm.value.name}`)
    showUploadModal.value = false
  } catch (e) {
    notificationStore.addNotification('seed_upload_failed', 'error', '上传失败', e instanceof Error ? e.message : '未知错误')
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleString()
}

// Watch for template selection
watch(() => generateForm.value.selectedTemplate, (newId) => {
  if (newId) {
    const template = seedStore.templates.find(t => t.id === newId)
    if (template) {
      generateForm.value.config = JSON.parse(JSON.stringify(template.config))
    }
  }
})

// Initialize
onMounted(async () => {
  await Promise.all([
    seedStore.fetchSeeds(),
    seedStore.fetchTemplates(),
  ])
})
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-2xl font-bold text-terminal-accent">种子管理</h1>
      <div class="flex gap-2">
        <button
          @click="openUploadModal"
          class="px-4 py-2 bg-terminal-surface border border-terminal-border rounded hover:border-terminal-accent transition-colors"
        >
          上传种子
        </button>
        <button
          @click="openGenerateModal()"
          class="px-4 py-2 bg-terminal-accent text-black rounded hover:bg-terminal-accent/80 transition-colors"
        >
          生成种子
        </button>
      </div>
    </div>

    <!-- Main content: left-right split -->
    <div class="flex-1 flex gap-4 min-h-0">
      <!-- Left panel: Seed list -->
      <div class="w-1/3 flex flex-col bg-terminal-surface border border-terminal-border rounded-lg overflow-hidden">
        <!-- Search -->
        <div class="p-3 border-b border-terminal-border">
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索种子..."
            class="w-full px-3 py-2 bg-terminal-bg border border-terminal-border rounded text-sm focus:border-terminal-accent focus:outline-none"
          />
        </div>

        <!-- Seed list -->
        <div class="flex-1 overflow-y-auto">
          <div v-if="seedStore.loading" class="p-4 text-center text-terminal-muted">
            加载中...
          </div>
          <div v-else-if="filteredSeeds.length === 0" class="p-4 text-center text-terminal-muted">
            暂无种子文件
          </div>
          <div v-else>
            <div
              v-for="seed in filteredSeeds"
              :key="seed.id"
              @click="selectSeed(seed)"
              class="p-3 border-b border-terminal-border cursor-pointer hover:bg-terminal-bg/50 transition-colors"
              :class="{ 'bg-terminal-accent/10': selectedSeedId === seed.id }"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span
                    class="w-2 h-2 rounded-full"
                    :class="seed.is_valid ? 'bg-terminal-success' : 'bg-terminal-error'"
                  ></span>
                  <span class="font-medium">{{ seed.name }}</span>
                </div>
                <span class="text-xs text-terminal-muted">{{ formatBytes(seed.file_size) }}</span>
              </div>
              <div class="mt-1 flex items-center justify-between text-xs text-terminal-muted">
                <span>使用 {{ seed.times_used }} 次 / 崩溃 {{ seed.crashes_found }}</span>
                <span>{{ formatDate(seed.created_at).split(' ')[0] }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Stats footer -->
        <div class="p-3 border-t border-terminal-border text-xs text-terminal-muted">
          共 {{ seedStore.totalSeeds }} 个种子
        </div>
      </div>

      <!-- Right panel: Details / Generate / Templates -->
      <div class="flex-1 flex flex-col bg-terminal-surface border border-terminal-border rounded-lg overflow-hidden">
        <!-- Tabs -->
        <div class="flex border-b border-terminal-border">
          <button
            @click="activeTab = 'details'"
            class="px-4 py-2 text-sm transition-colors"
            :class="activeTab === 'details' ? 'text-terminal-accent border-b-2 border-terminal-accent' : 'text-terminal-muted hover:text-terminal-text'"
          >
            详情
          </button>
          <button
            @click="activeTab = 'templates'"
            class="px-4 py-2 text-sm transition-colors"
            :class="activeTab === 'templates' ? 'text-terminal-accent border-b-2 border-terminal-accent' : 'text-terminal-muted hover:text-terminal-text'"
          >
            模板
          </button>
        </div>

        <!-- Tab content -->
        <div class="flex-1 overflow-y-auto p-4">
          <!-- Details tab -->
          <div v-if="activeTab === 'details'">
            <div v-if="!selectedSeed" class="text-center text-terminal-muted py-8">
              请从左侧选择一个种子查看详情
            </div>
            <div v-else>
              <!-- Seed info -->
              <div class="mb-6">
                <h2 class="text-lg font-semibold mb-2">{{ selectedSeed.name }}</h2>
                <div class="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span class="text-terminal-muted">文件大小:</span>
                    <span class="ml-2">{{ formatBytes(selectedSeed.file_size) }}</span>
                  </div>
                  <div>
                    <span class="text-terminal-muted">状态:</span>
                    <span class="ml-2" :class="selectedSeed.is_valid ? 'text-terminal-success' : 'text-terminal-error'">
                      {{ selectedSeed.is_valid ? '有效' : '无效' }}
                    </span>
                  </div>
                  <div>
                    <span class="text-terminal-muted">使用次数:</span>
                    <span class="ml-2">{{ selectedSeed.times_used }}</span>
                  </div>
                  <div>
                    <span class="text-terminal-muted">发现崩溃:</span>
                    <span class="ml-2">{{ selectedSeed.crashes_found }}</span>
                  </div>
                  <div class="col-span-2">
                    <span class="text-terminal-muted">校验和:</span>
                    <span class="ml-2 font-mono text-xs">{{ selectedSeed.checksum || 'N/A' }}</span>
                  </div>
                  <div class="col-span-2">
                    <span class="text-terminal-muted">创建时间:</span>
                    <span class="ml-2">{{ formatDate(selectedSeed.created_at) }}</span>
                  </div>
                </div>
              </div>

              <!-- Config -->
              <div class="mb-6">
                <h3 class="text-sm font-semibold text-terminal-muted mb-2">配置</h3>
                <div class="bg-terminal-bg p-3 rounded text-sm font-mono overflow-x-auto">
                  <pre>{{ JSON.stringify(selectedSeed.config, null, 2) }}</pre>
                </div>
              </div>

              <!-- Actions -->
              <div class="flex gap-2">
                <button
                  @click="downloadSeed(selectedSeed)"
                  class="px-4 py-2 bg-terminal-surface border border-terminal-border rounded hover:border-terminal-accent transition-colors"
                >
                  下载
                </button>
                <button
                  @click="confirmDelete(selectedSeed)"
                  class="px-4 py-2 bg-red-500/20 text-red-400 border border-red-500/30 rounded hover:bg-red-500/30 transition-colors"
                >
                  删除
                </button>
              </div>
            </div>
          </div>

          <!-- Templates tab -->
          <div v-if="activeTab === 'templates'">
            <div class="grid grid-cols-2 gap-4">
              <div
                v-for="template in seedStore.templates"
                :key="template.id"
                class="p-4 border border-terminal-border rounded-lg hover:border-terminal-accent cursor-pointer transition-colors"
                @click="openGenerateModal(template)"
              >
                <h3 class="font-semibold mb-1">{{ template.name }}</h3>
                <p class="text-sm text-terminal-muted mb-2">{{ template.description }}</p>
                <div class="text-xs text-terminal-muted">
                  {{ template.config.tags?.join(', ') || '无标签' }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Delete confirmation modal -->
    <div v-if="showDeleteConfirm" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div class="bg-terminal-surface border border-terminal-border rounded-lg p-6 max-w-sm">
        <h3 class="text-lg font-semibold mb-2">确认删除</h3>
        <p class="text-terminal-muted mb-4">
          确定要删除种子 "{{ selectedSeed?.name }}" 吗？此操作无法撤销。
        </p>
        <div class="flex justify-end gap-2">
          <button
            @click="showDeleteConfirm = false"
            class="px-4 py-2 bg-terminal-bg border border-terminal-border rounded hover:border-terminal-accent transition-colors"
          >
            取消
          </button>
          <button
            @click="deleteSeed"
            class="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
          >
            删除
          </button>
        </div>
      </div>
    </div>

    <!-- Generate modal -->
    <div v-if="showGenerateModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div class="bg-terminal-surface border border-terminal-border rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h3 class="text-lg font-semibold mb-4">生成种子</h3>

        <div class="space-y-4">
          <!-- Basic settings -->
          <div>
            <label class="block text-sm text-terminal-muted mb-1">种子名称</label>
            <input
              v-model="generateForm.name"
              type="text"
              class="w-full px-3 py-2 bg-terminal-bg border border-terminal-border rounded focus:border-terminal-accent focus:outline-none"
              placeholder="输入种子名称"
            />
          </div>

          <div>
            <label class="block text-sm text-terminal-muted mb-1">生成数量</label>
            <input
              v-model.number="generateForm.count"
              type="number"
              min="1"
              max="100"
              class="w-32 px-3 py-2 bg-terminal-bg border border-terminal-border rounded focus:border-terminal-accent focus:outline-none"
            />
          </div>

          <!-- Template selection -->
          <div>
            <label class="block text-sm text-terminal-muted mb-1">基于模板</label>
            <select
              v-model="generateForm.selectedTemplate"
              class="w-full px-3 py-2 bg-terminal-bg border border-terminal-border rounded focus:border-terminal-accent focus:outline-none"
            >
              <option :value="null">自定义配置</option>
              <option v-for="t in seedStore.templates" :key="t.id" :value="t.id">{{ t.name }}</option>
            </select>
          </div>

          <!-- Config JSON -->
          <div>
            <label class="block text-sm text-terminal-muted mb-1">配置 (JSON)</label>
            <textarea
              :value="JSON.stringify(generateForm.config, null, 2)"
              class="w-full h-64 px-3 py-2 bg-terminal-bg border border-terminal-border rounded font-mono text-sm focus:border-terminal-accent focus:outline-none"
              readonly
            ></textarea>
          </div>
        </div>

        <div class="flex justify-end gap-2 mt-6">
          <button
            @click="showGenerateModal = false"
            class="px-4 py-2 bg-terminal-bg border border-terminal-border rounded hover:border-terminal-accent transition-colors"
          >
            取消
          </button>
          <button
            @click="generateSeeds"
            :disabled="seedStore.loading"
            class="px-4 py-2 bg-terminal-accent text-black rounded hover:bg-terminal-accent/80 transition-colors disabled:opacity-50"
          >
            {{ seedStore.loading ? '生成中...' : '生成' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Upload modal -->
    <div v-if="showUploadModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div class="bg-terminal-surface border border-terminal-border rounded-lg p-6 max-w-md">
        <h3 class="text-lg font-semibold mb-4">上传种子</h3>

        <div class="space-y-4">
          <div>
            <label class="block text-sm text-terminal-muted mb-1">种子名称</label>
            <input
              v-model="uploadForm.name"
              type="text"
              class="w-full px-3 py-2 bg-terminal-bg border border-terminal-border rounded focus:border-terminal-accent focus:outline-none"
              placeholder="输入种子名称"
            />
          </div>

          <div>
            <label class="block text-sm text-terminal-muted mb-1">EROFS 文件</label>
            <input
              ref="fileInput"
              type="file"
              accept=".erofs"
              @change="handleFileSelect"
              class="w-full px-3 py-2 bg-terminal-bg border border-terminal-border rounded focus:border-terminal-accent focus:outline-none"
            />
          </div>
        </div>

        <div class="flex justify-end gap-2 mt-6">
          <button
            @click="showUploadModal = false"
            class="px-4 py-2 bg-terminal-bg border border-terminal-border rounded hover:border-terminal-accent transition-colors"
          >
            取消
          </button>
          <button
            @click="uploadSeed"
            :disabled="seedStore.loading"
            class="px-4 py-2 bg-terminal-accent text-black rounded hover:bg-terminal-accent/80 transition-colors disabled:opacity-50"
          >
            {{ seedStore.loading ? '上传中...' : '上传' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
