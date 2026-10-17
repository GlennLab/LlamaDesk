// LlamaDesk frontend contract mirror.
// These types mirror the frozen backend contract and must be kept in sync with it.
// All fields are camelCase. Missing values are `null` — never coerced to 0.

/** Placeholder rendered for any missing (null) value. */
export const PLACEHOLDER = '—'

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/** Reachability of the configured llama.cpp server. */
export type ConnectionState = 'connected' | 'disconnected' | 'error'

/** Response of `/health` (unknown shape folded into a status string). */
export interface ServerHealth {
  status: string
}

/**
 * Response of `/props`. Known fields are typed; `...extra` maps to an index
 * signature so unknown keys survive without weakening the typed fields.
 */
export interface ServerProps {
  role: string
  maxInstances: number | null
  modelsAutoload: boolean | null
  modelAlias: string | null
  modelPath: string | null
  buildInfo: string | null
  corsProxyEnabled: boolean | null
  [key: string]: unknown
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/** Three known states serialise as strings; unknown states as `{unknown}`. */
export type ModelStatus =
  | 'loaded'
  | 'unloaded'
  | 'loading'
  | { unknown: string }

/** Input/output modality lists reported by `/props` for a model. */
export interface ModelArchitecture {
  inputModalities: string[]
  outputModalities: string[]
}

/** A model entry from `/models`. */
export interface ModelInfo {
  id: string
  aliases: string[]
  tags: string[]
  status: ModelStatus
  statusArgs: string[]
  source: string | null
  canRemove: boolean
  contextLength: number | null
  architecture: ModelArchitecture | null
}

// ---------------------------------------------------------------------------
// Slots
// ---------------------------------------------------------------------------

/** Speculative-decoding counters for a slot's next token. */
export interface NextToken {
  hasNextToken: boolean | null
  hasNewLine: boolean | null
  /**
   * Tokens still to be decoded. A negative value (typically `-1` for idle
   * slots) is llama.cpp's "no limit" sentinel meaning unlimited — not a
   * negative count. `null` means the field is absent from the response.
   */
  nRemain: number | null
  nDecoded: number | null
}

/** A single server slot (`/slots`). */
export interface Slot {
  id: number
  nCtx: number | null
  speculative: boolean | null
  isProcessing: boolean
  idTask: number | null
  nPromptTokens: number | null
  nPromptTokensProcessed: number | null
  nPromptTokensCache: number | null
  nextToken: NextToken[]
  params: Record<string, unknown> | null
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/** Per-model metrics derived from `/metrics`. */
export interface ServerMetrics {
  promptTokensPerSec: number | null
  generationTokensPerSec: number | null
  activeRequests: number | null
  deferredRequests: number | null
  promptTokensTotal: number | null
  promptTokensCachedTotal: number | null
  tokensPredictedTotal: number | null
  nDecodeTotal: number | null
  nTokensMax: number | null
  kvCacheUsageRatio: number | null
  raw: Record<string, number>
}

/** Per-GPU stats from `nvidia-smi`. */
export interface GpuStats {
  index: number
  name: string
  utilizationPercent: number | null
  memoryTotalMib: number | null
  memoryUsedMib: number | null
  temperatureC: number | null
  powerDrawW: number | null
  powerLimitW: number | null
}

// ---------------------------------------------------------------------------
// History samples (persisted rows)
// ---------------------------------------------------------------------------

/** One persisted server-metric sample. */
export interface ServerSample {
  timestampMs: number
  promptTokensPerSec: number | null
  generationTokensPerSec: number | null
  activeRequests: number | null
  deferredRequests: number | null
  kvCacheUsageRatio: number | null
  promptTokensTotal: number | null
  tokensPredictedTotal: number | null
}

/** One persisted GPU sample. */
export interface GpuSample {
  timestampMs: number
  gpuIndex: number
  utilizationPercent: number | null
  memoryUsedMib: number | null
  memoryTotalMib: number | null
  temperatureC: number | null
  powerDrawW: number | null
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/** A non-fatal failure of one polling slice. */
export interface SliceError {
  slice: string
  message: string
}

/** Aggregate state emitted on the `monitor:snapshot` event. */
export interface MonitorSnapshot {
  connection: ConnectionState
  health: ServerHealth | null
  props: ServerProps | null
  models: ModelInfo[]
  loadedModels: string[]
  slots: Record<string, Slot[]>
  metrics: Record<string, ServerMetrics>
  gpus: GpuStats[]
  gpuAvailable: boolean
  serverUrl: string
  lastUpdated: string | null
  errors: SliceError[]
}

// ---------------------------------------------------------------------------
// Settings & diagnostics
// ---------------------------------------------------------------------------

/** Persisted application configuration. */
export interface AppConfig {
  serverUrl: string
  serverIntervalMs: number
  slotsIntervalMs: number
  hardwareIntervalMs: number
  storageIntervalMs: number
  retentionHours: number
}

/** Result of the `test_connection` command. */
export interface ConnectionTest {
  ok: boolean
  message: string
  role?: string
  buildInfo?: string
  modelCount?: number
}

/** Configuration used until settings have loaded from the backend. */
export const DEFAULT_CONFIG: AppConfig = {
  serverUrl: 'http://localhost:8080',
  serverIntervalMs: 2000,
  slotsIntervalMs: 1000,
  hardwareIntervalMs: 2000,
  storageIntervalMs: 5000,
  retentionHours: 24,
}

// ---------------------------------------------------------------------------
// Pure formatting helpers
// ---------------------------------------------------------------------------

/** Returns true only when a model's status is the known `loaded` state. */
export function isModelLoaded(m: ModelInfo): boolean {
  return m.status === 'loaded'
}

/** Formats a rate to one decimal with an optional unit. */
export function formatRate(v: number | null, unit = 'tok/s'): string {
  if (v === null || !Number.isFinite(v)) return PLACEHOLDER
  const n = v.toFixed(1)
  return unit ? `${n} ${unit}` : n
}

/** Formats a MiB value as GiB to one decimal. */
export function formatMiB(v: number | null): string {
  if (v === null || !Number.isFinite(v)) return PLACEHOLDER
  return `${(v / 1024).toFixed(1)} GiB`
}

/** Formats a numeric percentage to one decimal. */
export function formatPercent(v: number | null): string {
  if (v === null || !Number.isFinite(v)) return PLACEHOLDER
  return `${v.toFixed(1)}%`
}

/** Formats a Celsius temperature to one decimal. */
export function formatTemperature(v: number | null): string {
  if (v === null || !Number.isFinite(v)) return PLACEHOLDER
  return `${v.toFixed(1)}°C`
}

/** Formats power draw in watts, optionally against a limit. */
export function formatPower(w: number | null, limit: number | null = null): string {
  if (w === null || !Number.isFinite(w)) return PLACEHOLDER
  const draw = `${w.toFixed(0)} W`
  if (limit === null || !Number.isFinite(limit)) return draw
  return `${draw} / ${limit.toFixed(0)} W`
}

/** Formats an uptime duration given in seconds. */
export function formatUptime(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) return PLACEHOLDER
  const total = Math.floor(seconds)
  const days = Math.floor(total / 86400)
  const hours = Math.floor((total % 86400) / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const secs = total % 60
  if (days > 0) return `${days}d ${hours}h ${minutes}m`
  if (hours > 0) return `${hours}h ${minutes}m`
  if (minutes > 0) return `${minutes}m ${secs}s`
  return `${secs}s`
}

/** Formats an ISO timestamp as a local time, or the placeholder. */
export function formatTimestamp(iso: string | null): string {
  if (iso === null) return PLACEHOLDER
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return PLACEHOLDER
  return d.toLocaleTimeString()
}
