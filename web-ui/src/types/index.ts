// 任务状态
export type TaskStatus = 'pending' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled'

// 执行器类型
export type ExecutorType = 'erofsfuse' | 'qemu'

// 崩溃类型
export type CrashType = 'Signal' | 'ASan' | 'KernelPanic' | 'KernelOops'

// 变异器类型
export type MutatorType =
  | 'bitflip'
  | 'random'
  | 'zero'
  | 'max'
  | 'arithmetic'
  | 'interesting_values'
  | 'boundary'
  | 'superblock'
  | 'inode'
  | 'dirent'
  | 'xattr'
  | 'targeted'

// 层级类型
export type LayerType = 'superblock' | 'inode' | 'dirent' | 'data_block'

// 变异器配置
export interface MutatorConfig {
  enabled: boolean
  weight: number
  min_iterations?: number
  max_iterations?: number
  params?: Record<string, unknown>
}

// 目标字段
export interface TargetField {
  type: 'superblock' | 'inode' | 'dirent' | 'range' | 'data_block'
  field?: string
  index?: number
  part?: 'all' | 'nid' | 'name_off' | 'file_type' | 'name'
  start?: number
  length?: number
  block_num?: number
  offset_in_block?: number
}

// 层级配置
export interface LayerConfig {
  layer: LayerType
  mutators: Record<MutatorType, MutatorConfig>
  targets?: TargetField[]
}

// 自适应触发器
export interface AdaptiveTrigger {
  type: 'crash_found' | 'no_crash_iterations' | 'crash_type'
  count?: number
  crash_type?: string
}

// 自适应规则
export interface AdaptiveRule {
  trigger: AdaptiveTrigger
  mutator: MutatorType
  adjustment_percent: number
}

// 策略模板
export interface StrategyTemplate {
  id?: number
  name: string
  description: string
  is_builtin: boolean
  created_at?: string
  updated_at?: string
  mutators: Record<MutatorType, MutatorConfig>
  layers?: LayerConfig[]
  adaptive_rules?: AdaptiveRule[]
  adaptive_enabled: boolean
}

// 创建策略请求
export interface CreateStrategyRequest {
  name: string
  description?: string
  mutators: Record<MutatorType, MutatorConfig>
  layers?: LayerConfig[]
  adaptive_rules?: AdaptiveRule[]
  adaptive_enabled?: boolean
}

// 更新策略请求
export interface UpdateStrategyRequest {
  name?: string
  description?: string
  mutators?: Record<MutatorType, MutatorConfig>
  layers?: LayerConfig[]
  adaptive_rules?: AdaptiveRule[]
  adaptive_enabled?: boolean
}

// 变异器统计
export interface MutatorStats {
  mutator: MutatorType
  executions: number
  crashes: number
  current_weight: number
  original_weight: number
}

// 策略统计
export interface StrategyStats {
  task_id: number
  strategy_id?: number
  strategy_name: string
  mutators: MutatorStats[]
  total_iterations: number
  total_crashes: number
  adaptive_active: boolean
}

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
  strategy_id?: number
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
  strategy_id?: number
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

export interface StrategyStatsMessage {
  type: 'strategy_stats'
  task_id: number
  stats: StrategyStats
}

export type ServerMessage =
  | ProgressMessage
  | StatusMessage
  | CrashMessage
  | TaskCreatedMessage
  | ErrorMessage
  | StrategyStatsMessage

// API 响应
export interface ErrorResponse {
  error: string
}

// 导出响应
export interface ExportResponse {
  format: string
  content: string
}

// 变异器显示信息
export const MUTATOR_INFO: Record<MutatorType, { name: string; description: string; category: 'basic' | 'structure' | 'targeted' }> = {
  bitflip: {
    name: 'BitFlip',
    description: 'Random bit flipping',
    category: 'basic',
  },
  random: {
    name: 'Random',
    description: 'Random byte replacement',
    category: 'basic',
  },
  zero: {
    name: 'Zero',
    description: 'Set bytes to zero',
    category: 'basic',
  },
  max: {
    name: 'Max',
    description: 'Set bytes to 0xFF',
    category: 'basic',
  },
  arithmetic: {
    name: 'Arithmetic',
    description: 'Add/subtract small values',
    category: 'basic',
  },
  interesting_values: {
    name: 'Interesting Values',
    description: 'Use edge case values',
    category: 'basic',
  },
  boundary: {
    name: 'Boundary',
    description: 'Use boundary values',
    category: 'basic',
  },
  superblock: {
    name: 'Superblock',
    description: 'Structure-aware superblock mutation',
    category: 'structure',
  },
  inode: {
    name: 'Inode',
    description: 'Structure-aware inode mutation',
    category: 'structure',
  },
  dirent: {
    name: 'Directory Entry',
    description: 'Structure-aware dirent mutation',
    category: 'structure',
  },
  xattr: {
    name: 'Extended Attribute',
    description: 'Structure-aware xattr mutation',
    category: 'structure',
  },
  targeted: {
    name: 'Targeted',
    description: 'Precise field mutation',
    category: 'targeted',
  },
}

// 内置模板 ID
export const BUILTIN_TEMPLATE_IDS = [-1, -2, -3, -4]

// 模板名称
export const TEMPLATE_NAMES = {
  quick_discovery: 'Quick Discovery',
  structure_deep: 'Structure Deep',
  boundary_test: 'Boundary Test',
  full_coverage: 'Full Coverage',
}
