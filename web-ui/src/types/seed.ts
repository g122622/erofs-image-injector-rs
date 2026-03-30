// Seed Management Types

// Compression algorithm
export type CompressionAlgorithm = 'lz4' | 'lz4hc' | 'lzma' | 'zstd'

// Compression configuration
export interface CompressionConfig {
  algorithm: CompressionAlgorithm
  level?: number
  dict_size?: number
}

// Node type
export type NodeType = 'file' | 'directory' | 'symlink'

// File content type
export type FileContentType = 'text' | 'binary' | 'afl_generated' | 'random' | 'pattern'

// Entropy level
export type EntropyLevel = 'low' | 'medium' | 'high'

// AFL format
export type AflFormat = 'raw' | 'afl'

// AFL content configuration
export interface AflContentConfig {
  size_range: [number, number]
  with_header?: boolean
  format?: AflFormat
  pattern_injection?: string
}

// Random content configuration
export interface RandomContentConfig {
  size_range: [number, number]
  entropy?: EntropyLevel
}

// Pattern content configuration
export interface PatternContentConfig {
  pattern: string
  repeat_count?: number
  size?: number
}

// File content configuration
export interface FileContentConfig {
  type: FileContentType
  text_content?: string
  binary_content?: string
  afl_config?: AflContentConfig
  random_config?: RandomContentConfig
  pattern_config?: PatternContentConfig
}

// Extended attribute
export interface ExtendedAttribute {
  name: string
  value: string
}

// Directory tree node
export interface DirectoryTreeNode {
  name: string
  type: NodeType
  content?: FileContentConfig
  children?: DirectoryTreeNode[]
  xattr?: ExtendedAttribute[]
  mode?: number
  uid?: number
  gid?: number
  target?: string
}

// Seed configuration
export interface SeedConfig {
  block_size: number
  volume_name: string
  compression?: CompressionConfig
  root: DirectoryTreeNode
  description?: string
  tags?: string[]
}

// Seed model
export interface Seed {
  id: number
  name: string
  file_path: string
  file_size: number
  checksum?: string
  config: SeedConfig
  times_used: number
  crashes_found: number
  created_at: string
  updated_at?: string
  is_valid: boolean
  tags?: string
}

// Seed filter for querying
export interface SeedFilter {
  is_valid?: boolean
  tag?: string
  limit?: number
  offset?: number
}

// Create seed request
export interface CreateSeedRequest {
  name: string
  config: SeedConfig
  count?: number
}

// Seed generation job status
export interface SeedGenerationJob {
  id: number
  status: 'pending' | 'running' | 'completed' | 'failed'
  progress: number
  seeds_generated: number
  seeds_total: number
  error_message?: string
}

// Seed template
export interface SeedTemplate {
  id: string
  name: string
  description: string
  config: SeedConfig
}

// Seed statistics for a task
export interface SeedTaskStats {
  seed_id: number
  seed_name: string
  seed_index: number
  iterations: number
  crashes: number
}

// WebSocket message for seed info
export interface SeedInfoMessage {
  type: 'seed_info'
  task_id: number
  current_seed?: string
  seed_index: number
  total_seeds: number
}

// WebSocket message for seed generation progress
export interface SeedGeneratedMessage {
  type: 'seed_generated'
  job_id: number
  seed_name: string
  seed_id: number
  progress: number
}

// Default seed config
export const DEFAULT_SEED_CONFIG: SeedConfig = {
  block_size: 4096,
  volume_name: 'erofs',
  root: {
    name: 'root',
    type: 'directory',
    children: [],
    mode: 0o755,
    uid: 0,
    gid: 0,
  },
}

// Compression algorithm options
export const COMPRESSION_ALGORITHMS: { value: CompressionAlgorithm; label: string; description: string }[] = [
  { value: 'lz4', label: 'LZ4', description: 'Fast compression, good for fuzzing' },
  { value: 'lz4hc', label: 'LZ4HC', description: 'Higher compression ratio LZ4' },
  { value: 'lzma', label: 'LZMA', description: 'High compression ratio, slower' },
  { value: 'zstd', label: 'ZSTD', description: 'Balanced compression and speed' },
]

// Block size options
export const BLOCK_SIZES: { value: number; label: string }[] = [
  { value: 512, label: '512 bytes' },
  { value: 1024, label: '1 KB' },
  { value: 2048, label: '2 KB' },
  { value: 4096, label: '4 KB (default)' },
]

// Entropy level options
export const ENTROPY_LEVELS: { value: EntropyLevel; label: string; description: string }[] = [
  { value: 'low', label: 'Low', description: 'Mostly zeros with some random bytes' },
  { value: 'medium', label: 'Medium', description: 'Patterned random data' },
  { value: 'high', label: 'High', description: 'Fully random data' },
]
