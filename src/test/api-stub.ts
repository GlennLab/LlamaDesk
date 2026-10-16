// Shared spies for the Tauri IPC boundary (`src/lib/api`).
//
// Every UI test mocks `../lib/api` with `apiMock()` so the real hooks, providers
// and components run unchanged against a deterministic bridge — no Tauri, no
// network. `describeError` is preserved from the real module so error paths
// exercise production code.
import { vi } from 'vitest'
import { DEFAULT_CONFIG } from '../types'
import type {
  AppConfig,
  ConnectionTest,
  GpuSample,
  MonitorSnapshot,
  ServerSample,
} from '../types'
import { makeSnapshot } from './fixtures'

/** The bridge spies. Referenced by `apiMock` and reset between tests. */
export const api = {
  getSnapshot: vi.fn<() => Promise<MonitorSnapshot>>(),
  getSettings: vi.fn<() => Promise<AppConfig>>(),
  subscribeSnapshot: vi.fn<() => Promise<() => void>>(),
  updateSettings: vi.fn<(config: AppConfig) => Promise<AppConfig>>(),
  testConnection: vi.fn<(url?: string) => Promise<ConnectionTest>>(),
  getServerHistory: vi.fn<(fromMs: number, toMs: number) => Promise<ServerSample[]>>(),
  getGpuHistory: vi.fn<(fromMs: number, toMs: number) => Promise<GpuSample[]>>(),
  pruneHistory: vi.fn<() => Promise<number>>(),
}

/** Restores every spy and installs inert defaults. Call in `beforeEach`. */
export function resetApi(): void {
  api.getSnapshot.mockReset().mockResolvedValue(makeSnapshot())
  api.getSettings.mockReset().mockResolvedValue({ ...DEFAULT_CONFIG })
  api.subscribeSnapshot.mockReset().mockResolvedValue(() => {})
  api.updateSettings.mockReset()
  api.testConnection.mockReset()
  api.getServerHistory.mockReset().mockResolvedValue([])
  api.getGpuHistory.mockReset().mockResolvedValue([])
  api.pruneHistory.mockReset()
}

/**
 * Builds the mocked `../lib/api` module for a `vi.mock` factory, keeping the
 * real helpers (e.g. `describeError`) and replacing the IPC-touching functions.
 */
export async function apiMock() {
  const actual = await vi.importActual<typeof import('../lib/api')>('../lib/api')
  return { ...actual, ...api }
}
