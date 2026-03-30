import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  Seed,
  SeedFilter,
  CreateSeedRequest,
  SeedTemplate,
  SeedInfoMessage
} from '@/types/seed'

const API_BASE = '/api'

// API client for seeds
class SeedApiClient {
  private async request<T>(path: string, options?: RequestInit): Promise<T> {
    const response = await fetch(`${API_BASE}${path}`, {
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      ...options,
    })

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }))
      throw new Error(error.error || `HTTP ${response.status}`)
    }

    if (response.status === 204) {
      return undefined as T
    }

    return response.json()
  }

  async listSeeds(filter?: SeedFilter): Promise<Seed[]> {
    const params = new URLSearchParams()
    if (filter?.is_valid !== undefined) params.append('is_valid', String(filter.is_valid))
    if (filter?.tag) params.append('tag', filter.tag)
    if (filter?.limit) params.append('limit', String(filter.limit))
    if (filter?.offset) params.append('offset', String(filter.offset))
    const query = params.toString()
    return this.request<Seed[]>(`/seeds${query ? '?' + query : ''}`)
  }

  async getSeed(id: number): Promise<Seed> {
    return this.request<Seed>(`/seeds/${id}`)
  }

  async generateSeeds(request: CreateSeedRequest): Promise<Seed[]> {
    return this.request<Seed[]>('/seeds/generate', {
      method: 'POST',
      body: JSON.stringify(request),
    })
  }

  async uploadSeed(name: string, config: Record<string, unknown>, file: File): Promise<Seed> {
    const formData = new FormData()
    formData.append('name', name)
    formData.append('config', JSON.stringify(config))
    formData.append('file', file)

    const response = await fetch(`${API_BASE}/seeds/upload`, {
      method: 'POST',
      body: formData,
    })

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }))
      throw new Error(error.error || `HTTP ${response.status}`)
    }

    return response.json()
  }

  async deleteSeed(id: number): Promise<void> {
    await this.request(`/seeds/${id}`, { method: 'DELETE' })
  }

  async downloadSeed(id: number): Promise<Blob> {
    const response = await fetch(`${API_BASE}/seeds/${id}/download`)
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }))
      throw new Error(error.error || `HTTP ${response.status}`)
    }
    return response.blob()
  }

  async listTemplates(): Promise<SeedTemplate[]> {
    return this.request<SeedTemplate[]>('/seeds/templates')
  }

  async getTemplate(id: string): Promise<SeedTemplate> {
    return this.request<SeedTemplate>(`/seeds/templates/${id}`)
  }
}

const seedApi = new SeedApiClient()

export const useSeedStore = defineStore('seed', () => {
  // State
  const seeds = ref<Seed[]>([])
  const templates = ref<SeedTemplate[]>([])
  const currentSeed = ref<Seed | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Seed tracking during task execution
  const currentSeedInfo = ref<SeedInfoMessage | null>(null)

  // Computed
  const validSeeds = computed(() => seeds.value.filter(s => s.is_valid))
  const totalSeeds = computed(() => seeds.value.length)

  // Actions
  async function fetchSeeds(filter?: SeedFilter) {
    loading.value = true
    error.value = null
    try {
      seeds.value = await seedApi.listSeeds(filter)
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch seeds'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchSeed(id: number) {
    loading.value = true
    error.value = null
    try {
      currentSeed.value = await seedApi.getSeed(id)
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch seed'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function generateSeeds(request: CreateSeedRequest) {
    loading.value = true
    error.value = null
    try {
      const newSeeds = await seedApi.generateSeeds(request)
      seeds.value = [...seeds.value, ...newSeeds]
      return newSeeds
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to generate seeds'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function uploadSeed(name: string, config: Record<string, unknown>, file: File) {
    loading.value = true
    error.value = null
    try {
      const seed = await seedApi.uploadSeed(name, config, file)
      seeds.value.push(seed)
      return seed
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to upload seed'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteSeed(id: number) {
    loading.value = true
    error.value = null
    try {
      await seedApi.deleteSeed(id)
      seeds.value = seeds.value.filter(s => s.id !== id)
      if (currentSeed.value?.id === id) {
        currentSeed.value = null
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to delete seed'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function downloadSeed(id: number) {
    const blob = await seedApi.downloadSeed(id)
    const seed = seeds.value.find(s => s.id === id)
    const filename = seed ? `${seed.name}.erofs` : `seed-${id}.erofs`

    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  async function fetchTemplates() {
    loading.value = true
    error.value = null
    try {
      templates.value = await seedApi.listTemplates()
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch templates'
      throw e
    } finally {
      loading.value = false
    }
  }

  function updateSeedInfo(info: SeedInfoMessage) {
    currentSeedInfo.value = info
  }

  function clearSeedInfo() {
    currentSeedInfo.value = null
  }

  return {
    // State
    seeds,
    templates,
    currentSeed,
    loading,
    error,
    currentSeedInfo,

    // Computed
    validSeeds,
    totalSeeds,

    // Actions
    fetchSeeds,
    fetchSeed,
    generateSeeds,
    uploadSeed,
    deleteSeed,
    downloadSeed,
    fetchTemplates,
    updateSeedInfo,
    clearSeedInfo,
  }
})
