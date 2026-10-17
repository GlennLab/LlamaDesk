// Thin, typed wrapper over the Tauri IPC boundary.
// Command and event names are a frozen public contract shared with the backend.
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppConfig,
  ConnectionTest,
  GpuSample,
  MonitorSnapshot,
  ServerSample,
} from '../types'

/** Turns a Tauri rejection (usually an `Err(String)`) into a readable string. */
export function describeError(e: unknown): string {
  if (typeof e === 'string') return e
  if (e instanceof Error) return e.message
  return String(e)
}

/** Fetches the current aggregate snapshot. */
export async function getSnapshot(): Promise<MonitorSnapshot> {
  return invoke<MonitorSnapshot>('get_snapshot')
}

/** Fetches the persisted application configuration. */
export async function getSettings(): Promise<AppConfig> {
  return invoke<AppConfig>('get_settings')
}

/** Validates, persists and applies the configuration; returns the effective one. */
export async function updateSettings(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>('update_settings', { config })
}

/** Tests connectivity to the server, optionally overriding the configured URL. */
export async function testConnection(url?: string): Promise<ConnectionTest> {
  return invoke<ConnectionTest>('test_connection', { url })
}

/** Reads persisted server-metric samples within a millisecond range. */
export async function getServerHistory(fromMs: number, toMs: number): Promise<ServerSample[]> {
  return invoke<ServerSample[]>('get_server_history', { fromMs, toMs })
}

/** Reads persisted GPU samples within a millisecond range. */
export async function getGpuHistory(fromMs: number, toMs: number): Promise<GpuSample[]> {
  return invoke<GpuSample[]>('get_gpu_history', { fromMs, toMs })
}

/** Prunes history beyond retention; returns the number of rows removed. */
export async function pruneHistory(): Promise<number> {
  return invoke<number>('prune_history')
}

/** Subscribes to the `monitor:snapshot` event; resolves to an unlisten handle. */
export function subscribeSnapshot(
  cb: (s: MonitorSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<MonitorSnapshot>('monitor:snapshot', (e) => cb(e.payload))
}
