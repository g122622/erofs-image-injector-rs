import type {
  Task,
  TaskConfig,
  Crash,
  TaskStats,
  CrashType,
  StrategyTemplate,
  CreateStrategyRequest,
  UpdateStrategyRequest,
  ExportResponse,
} from '@/types'

const API_BASE = '/api'

// API 客户端
class ApiClient {
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

    return response.json()
  }

  // 任务 API
  async listTasks(): Promise<Task[]> {
    return this.request<Task[]>('/tasks')
  }

  async getTask(id: number): Promise<Task> {
    return this.request<Task>(`/tasks/${id}`)
  }

  async createTask(config: TaskConfig): Promise<Task> {
    return this.request<Task>('/tasks', {
      method: 'POST',
      body: JSON.stringify(config),
    })
  }

  async startTask(id: number): Promise<Task> {
    return this.request<Task>(`/tasks/${id}/start`, { method: 'POST' })
  }

  async stopTask(id: number): Promise<Task> {
    return this.request<Task>(`/tasks/${id}/stop`, { method: 'POST' })
  }

  async pauseTask(id: number): Promise<Task> {
    return this.request<Task>(`/tasks/${id}/pause`, { method: 'POST' })
  }

  async resumeTask(id: number): Promise<Task> {
    return this.request<Task>(`/tasks/${id}/resume`, { method: 'POST' })
  }

  async deleteTask(id: number): Promise<void> {
    await this.request(`/tasks/${id}`, { method: 'DELETE' })
  }

  // 崩溃 API
  async listCrashes(filter?: { task_id?: number; crash_type?: CrashType; limit?: number }): Promise<Crash[]> {
    const params = new URLSearchParams()
    if (filter?.task_id) params.append('task_id', String(filter.task_id))
    if (filter?.crash_type) params.append('crash_type', filter.crash_type)
    if (filter?.limit) params.append('limit', String(filter.limit))
    const query = params.toString()
    return this.request<Crash[]>(`/crashes${query ? '?' + query : ''}`)
  }

  async getCrash(id: number): Promise<Crash> {
    return this.request<Crash>(`/crashes/${id}`)
  }

  async getCrashRepro(id: number): Promise<{ script: string; script_type: string; description: string }> {
    return this.request(`/crashes/${id}/repro`)
  }

  // 统计 API
  async getStats(): Promise<TaskStats> {
    return this.request<TaskStats>('/stats')
  }

  // 健康检查
  async healthCheck(): Promise<{ status: string; version: string }> {
    return this.request('/health')
  }

  // 策略模板 API
  async listStrategies(): Promise<StrategyTemplate[]> {
    return this.request<StrategyTemplate[]>('/strategies')
  }

  async getStrategy(id: number): Promise<StrategyTemplate> {
    return this.request<StrategyTemplate>(`/strategies/${id}`)
  }

  async createStrategy(request: CreateStrategyRequest): Promise<StrategyTemplate> {
    return this.request<StrategyTemplate>('/strategies', {
      method: 'POST',
      body: JSON.stringify(request),
    })
  }

  async updateStrategy(id: number, request: UpdateStrategyRequest): Promise<StrategyTemplate> {
    return this.request<StrategyTemplate>(`/strategies/${id}`, {
      method: 'PUT',
      body: JSON.stringify(request),
    })
  }

  async deleteStrategy(id: number): Promise<void> {
    await this.request(`/strategies/${id}`, { method: 'DELETE' })
  }

  async duplicateStrategy(id: number, name?: string): Promise<StrategyTemplate> {
    return this.request<StrategyTemplate>(`/strategies/${id}/duplicate`, {
      method: 'POST',
      body: JSON.stringify({ name }),
    })
  }

  async exportStrategy(id: number): Promise<ExportResponse> {
    return this.request<ExportResponse>(`/strategies/${id}/export`)
  }

  async importStrategy(content: string): Promise<StrategyTemplate> {
    return this.request<StrategyTemplate>('/strategies/import', {
      method: 'POST',
      body: JSON.stringify({ content }),
    })
  }

  async importStrategyFile(file: File): Promise<StrategyTemplate> {
    const formData = new FormData()
    formData.append('file', file)

    const response = await fetch(`${API_BASE}/strategies/import-file`, {
      method: 'POST',
      body: formData,
    })

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }))
      throw new Error(error.error || `HTTP ${response.status}`)
    }

    return response.json()
  }
}

export const api = new ApiClient()
