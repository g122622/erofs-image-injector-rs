// 任务状态
export type TaskStatus = 'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'

// 执行器类型
export type ExecutorType = 'erofsfuse' | 'qemu'

// 崩溃类型
export type CrashType = 'Signal' | 'ASan' | 'KernelPanic' | 'KernelOops'

// 任务配置
export interface TaskConfig {
  name: string
  executor_type: ExecutorType
  seeds_dir: string
  output_dir: string
  timeout_seconds: number
  max_iterations: number
  workers?: number
  qemu_memory?: number
  kernel_path?: string
  initramfs_path?: string
  qemu_path?: string
  erofsfuse_path?: string
}

// 任务
export interface Task {
  id: number
  name: string
  status: TaskStatus
  executor_type: ExecutorType
  seeds_dir: string
  output_dir: string
  timeout_seconds: number
  max_iterations: number
  workers: number
  qemu_memory?: number
  kernel_path?: string
  initramfs_path?: string
  qemu_path?: string
  erofsfuse_path?: string
  current_iteration: number
  total_crashes: number
  exec_per_sec: number
  created_at: string
  started_at?: string
  finished_at?: string
  error_message?: string
}

// 崩溃
export interface Crash {
  id: number
  task_id: number
  iteration: number
  crash_type: CrashType
  signal?: number
  image_path: string
  log_path?: string
  created_at: string
}

// 统计
export interface TaskStats {
  total: number
  running: number
  pending: number
  completed: number
  failed: number
  total_crashes: number
  total_iterations: number
}

// WebSocket 消息
export interface ProgressMessage {
  type: 'progress'
  task_id: number
  iteration: number
  crashes: number
  speed: number
}

export interface StatusMessage {
  type: 'status'
  task_id: number
  status: TaskStatus
}

export interface CrashMessage {
  type: 'crash'
  task_id: number
  crash_id: number
  crash_type: CrashType
  iteration: number
}

export interface TaskCreatedMessage {
  type: 'task_created'
  task_id: number
}

export interface ErrorMessage {
  type: 'error'
  message: string
}

export type ServerMessage = ProgressMessage | StatusMessage | CrashMessage | TaskCreatedMessage | ErrorMessage

// API 响应
export interface ErrorResponse {
  error: string
}
