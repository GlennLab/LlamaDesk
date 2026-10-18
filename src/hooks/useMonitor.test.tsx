import { act, render, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { AppConfig, GpuSample, MonitorSnapshot, ServerSample } from '../types'
import {
  SnapshotProvider,
  useHistory,
  useSettings,
  useSnapshot,
  useSnapshotContext,
} from './useMonitor'

// Hoisted so the vi.mock factory can reference the spies.
const api = vi.hoisted(() => ({
  getSnapshot: vi.fn(),
  getSettings: vi.fn(),
  subscribeSnapshot: vi.fn(),
  updateSettings: vi.fn(),
  testConnection: vi.fn(),
  getServerHistory: vi.fn(),
  getGpuHistory: vi.fn(),
}))

// Keep the real describeError so error strings exercise the production path.
vi.mock('../lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/api')>()
  return {
    ...actual,
    getSnapshot: api.getSnapshot,
    getSettings: api.getSettings,
    subscribeSnapshot: api.subscribeSnapshot,
    updateSettings: api.updateSettings,
    testConnection: api.testConnection,
    getServerHistory: api.getServerHistory,
    getGpuHistory: api.getGpuHistory,
  }
})

const CONFIG: AppConfig = {
  serverUrl: 'http://localhost:8080',
  profiles: [{ name: 'Default', serverUrl: 'http://localhost:8080' }],
  activeProfile: 'Default',
  serverIntervalMs: 2000,
  slotsIntervalMs: 1000,
  hardwareIntervalMs: 2000,
  storageIntervalMs: 5000,
  retentionHours: 24,
}

function snapshot(overrides: Partial<MonitorSnapshot> = {}): MonitorSnapshot {
  return {
    connection: 'connected',
    health: { status: 'ok' },
    props: null,
    models: [],
    loadedModels: [],
    slots: {},
    metrics: {},
    gpus: [],
    gpuAvailable: false,
    serverUrl: 'http://localhost:8080',
    lastUpdated: null,
    errors: [],
    ...overrides,
  }
}

/** Flushes pending microtasks (resolved promises) inside act. */
const flush = (): Promise<void> => act(async () => {})

beforeEach(() => {
  Object.values(api).forEach((fn) => fn.mockReset())
})

describe('useSnapshot', () => {
  it('should populate the snapshot from get_snapshot on mount', async () => {
    // Arrange
    const snap = snapshot()
    api.getSnapshot.mockResolvedValue(snap)
    api.subscribeSnapshot.mockResolvedValue(vi.fn())

    // Act
    const { result } = renderHook(() => useSnapshot())

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.snapshot).toBe(snap)
    expect(result.current.error).toBeNull()
  })

  it('should update the snapshot when a monitor:snapshot event arrives', async () => {
    // Arrange
    const first = snapshot()
    const second = snapshot({ connection: 'disconnected' })
    api.getSnapshot.mockResolvedValue(first)
    let push: ((s: MonitorSnapshot) => void) | undefined
    api.subscribeSnapshot.mockImplementation((cb: (s: MonitorSnapshot) => void) => {
      push = cb
      return Promise.resolve(vi.fn())
    })

    const { result } = renderHook(() => useSnapshot())
    await waitFor(() => expect(result.current.snapshot).toBe(first))

    // Act
    act(() => {
      push?.(second)
    })

    // Assert
    expect(result.current.snapshot).toBe(second)
    expect(result.current.error).toBeNull()
    expect(result.current.loading).toBe(false)
  })

  it('should unsubscribe the monitor:snapshot listener on unmount', async () => {
    // Arrange
    const unlisten = vi.fn()
    api.getSnapshot.mockResolvedValue(snapshot())
    api.subscribeSnapshot.mockResolvedValue(unlisten)
    const { unmount } = renderHook(() => useSnapshot())
    await flush()

    // Act
    unmount()

    // Assert
    expect(unlisten).toHaveBeenCalledTimes(1)
  })

  it('should unsubscribe immediately when the subscription resolves after unmount', async () => {
    // Arrange
    let resolveSub: (fn: () => void) => void = () => {}
    api.getSnapshot.mockResolvedValue(snapshot())
    api.subscribeSnapshot.mockReturnValue(
      new Promise<() => void>((res) => {
        resolveSub = res
      }),
    )
    const unlisten = vi.fn()
    const { unmount } = renderHook(() => useSnapshot())
    unmount()

    // Act
    await act(async () => {
      resolveSub(unlisten)
    })

    // Assert
    expect(unlisten).toHaveBeenCalledTimes(1)
  })

  it('should ignore a monitor:snapshot event that arrives after unmount', async () => {
    // Arrange
    const first = snapshot()
    const late = snapshot({ connection: 'disconnected' })
    api.getSnapshot.mockResolvedValue(first)
    let push: ((s: MonitorSnapshot) => void) | undefined
    api.subscribeSnapshot.mockImplementation((cb: (s: MonitorSnapshot) => void) => {
      push = cb
      return Promise.resolve(vi.fn())
    })
    const { result, unmount } = renderHook(() => useSnapshot())
    await waitFor(() => expect(result.current.snapshot).toBe(first))
    unmount()

    // Act
    act(() => {
      push?.(late)
    })

    // Assert — the guard prevents a post-unmount state write
    expect(result.current.snapshot).toBe(first)
  })

  it('should not update state when get_snapshot rejects after unmount', async () => {
    // Arrange
    let rejectGet: (reason: unknown) => void = () => {}
    api.getSnapshot.mockReturnValue(
      new Promise<MonitorSnapshot>((_res, rej) => {
        rejectGet = rej
      }),
    )
    api.subscribeSnapshot.mockResolvedValue(vi.fn())
    const { unmount } = renderHook(() => useSnapshot())
    unmount()

    // Act & Assert — the rejection is swallowed by the unmount guard
    await act(async () => {
      rejectGet('late failure')
    })
  })

  it('should surface an error when the initial get_snapshot call fails', async () => {
    // Arrange
    api.getSnapshot.mockRejectedValue('server unreachable')
    api.subscribeSnapshot.mockResolvedValue(vi.fn())

    // Act
    const { result } = renderHook(() => useSnapshot())

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.error).toBe('server unreachable')
    expect(result.current.snapshot).toBeNull()
  })

  it('should surface an error when the subscription fails', async () => {
    // Arrange
    api.getSnapshot.mockResolvedValue(snapshot())
    api.subscribeSnapshot.mockRejectedValue(new Error('ipc unavailable'))

    // Act
    const { result } = renderHook(() => useSnapshot())

    // Assert
    await waitFor(() => expect(result.current.error).toBe('ipc unavailable'))
  })
})

describe('SnapshotProvider and useSnapshotContext', () => {
  it('should deliver the same snapshot to every consumer within the provider', async () => {
    // Arrange
    const snap = snapshot()
    api.getSnapshot.mockResolvedValue(snap)
    api.subscribeSnapshot.mockResolvedValue(vi.fn())
    const seenA: Array<MonitorSnapshot | null> = []
    const seenB: Array<MonitorSnapshot | null> = []

    function ConsumerA(): null {
      seenA.push(useSnapshotContext().snapshot)
      return null
    }
    function ConsumerB(): null {
      seenB.push(useSnapshotContext().snapshot)
      return null
    }

    // Act
    render(
      <SnapshotProvider>
        <ConsumerA />
        <ConsumerB />
      </SnapshotProvider>,
    )

    // Assert
    await waitFor(() => expect(seenA).toContain(snap))
    expect(seenA[seenA.length - 1]).toBe(snap)
    expect(seenB[seenB.length - 1]).toBe(snap)
  })

  it('should throw when useSnapshotContext is used outside a SnapshotProvider', () => {
    // Arrange
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
    function Orphan(): null {
      useSnapshotContext()
      return null
    }

    // Act & Assert
    try {
      expect(() => render(<Orphan />)).toThrow(
        'useSnapshotContext must be used within a SnapshotProvider',
      )
    } finally {
      consoleError.mockRestore()
    }
  })
})

describe('useSettings', () => {
  it('should load the persisted config on mount', async () => {
    // Arrange
    api.getSettings.mockResolvedValue(CONFIG)

    // Act
    const { result } = renderHook(() => useSettings())

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.config).toEqual(CONFIG)
    expect(result.current.error).toBeNull()
  })

  it('should persist and apply the config when save succeeds', async () => {
    // Arrange
    const next: AppConfig = { ...CONFIG, serverIntervalMs: 3000 }
    const persisted: AppConfig = { ...next, retentionHours: 48 }
    api.getSettings.mockResolvedValue(CONFIG)
    api.updateSettings.mockResolvedValue(persisted)
    const { result } = renderHook(() => useSettings())
    await waitFor(() => expect(result.current.loading).toBe(false))

    // Act
    await act(async () => {
      await result.current.save(next)
    })

    // Assert
    expect(api.updateSettings).toHaveBeenCalledWith(next)
    expect(result.current.config).toEqual(persisted)
    expect(result.current.saving).toBe(false)
    expect(result.current.error).toBeNull()
  })

  it('should surface and rethrow when save fails', async () => {
    // Arrange
    api.getSettings.mockResolvedValue(CONFIG)
    api.updateSettings.mockRejectedValue('invalid server url')
    const { result } = renderHook(() => useSettings())
    await waitFor(() => expect(result.current.loading).toBe(false))
    let caught: unknown

    // Act
    await act(async () => {
      try {
        await result.current.save(CONFIG)
      } catch (e) {
        caught = e
      }
    })

    // Assert
    expect((caught as Error).message).toBe('invalid server url')
    expect(result.current.error).toBe('invalid server url')
    expect(result.current.saving).toBe(false)
  })

  it('should probe the server through test_connection with the override url', async () => {
    // Arrange
    const probe = { ok: true, message: 'reachable' }
    api.getSettings.mockResolvedValue(CONFIG)
    api.testConnection.mockResolvedValue(probe)
    const { result } = renderHook(() => useSettings())
    await waitFor(() => expect(result.current.loading).toBe(false))
    let received: unknown

    // Act
    await act(async () => {
      received = await result.current.testUrl('http://example.test:9000')
    })

    // Assert
    expect(api.testConnection).toHaveBeenCalledWith('http://example.test:9000')
    expect(received).toEqual(probe)
  })

  it('should surface an error when settings fail to load', async () => {
    // Arrange
    api.getSettings.mockRejectedValue('config missing')

    // Act
    const { result } = renderHook(() => useSettings())

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.error).toBe('config missing')
    expect(result.current.config).toBeNull()
  })

  it('should not update state when get_settings resolves after unmount', async () => {
    // Arrange
    let resolveGet: (c: AppConfig) => void = () => {}
    api.getSettings.mockReturnValue(
      new Promise<AppConfig>((res) => {
        resolveGet = res
      }),
    )
    const { unmount } = renderHook(() => useSettings())
    unmount()

    // Act & Assert — the unmount guard swallows the late resolution
    await act(async () => {
      resolveGet(CONFIG)
    })
  })
})

describe('useHistory', () => {
  it('should fetch server history when kind is server', async () => {
    // Arrange
    const rows: ServerSample[] = [
      {
        timestampMs: 1000,
        promptTokensPerSec: 12.5,
        generationTokensPerSec: null,
        activeRequests: null,
        deferredRequests: null,
        kvCacheUsageRatio: null,
        promptTokensTotal: null,
        tokensPredictedTotal: null,
      },
    ]
    api.getServerHistory.mockResolvedValue(rows)

    // Act
    const { result } = renderHook(() => useHistory('server', 1000, 2000))

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(api.getServerHistory).toHaveBeenCalledWith(1000, 2000)
    expect(api.getGpuHistory).not.toHaveBeenCalled()
    expect(result.current.samples).toEqual(rows)
    expect(result.current.error).toBeNull()
  })

  it('should fetch GPU history when kind is gpu', async () => {
    // Arrange
    const rows: GpuSample[] = [
      {
        timestampMs: 1000,
        gpuIndex: 0,
        utilizationPercent: null,
        memoryUsedMib: null,
        memoryTotalMib: null,
        temperatureC: null,
        powerDrawW: null,
      },
    ]
    api.getGpuHistory.mockResolvedValue(rows)

    // Act
    const { result } = renderHook(() => useHistory('gpu', 1000, 2000))

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(api.getGpuHistory).toHaveBeenCalledWith(1000, 2000)
    expect(api.getServerHistory).not.toHaveBeenCalled()
    expect(result.current.samples).toEqual(rows)
  })

  it('should surface an error when history fails to load', async () => {
    // Arrange
    api.getServerHistory.mockRejectedValue('database is locked')

    // Act
    const { result } = renderHook(() => useHistory('server', 0, 1000))

    // Assert
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.error).toBe('database is locked')
    expect(result.current.samples).toEqual([])
  })

  it('should refetch when reload is called', async () => {
    // Arrange
    api.getServerHistory.mockResolvedValue([])
    const { result } = renderHook(() => useHistory('server', 0, 1000))
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(api.getServerHistory).toHaveBeenCalledTimes(1)

    // Act
    act(() => {
      result.current.reload()
    })

    // Assert
    await waitFor(() => expect(api.getServerHistory).toHaveBeenCalledTimes(2))
  })

  it('should not update state when history resolves after unmount', async () => {
    // Arrange
    let resolveHistory: (rows: ServerSample[]) => void = () => {}
    api.getServerHistory.mockReturnValue(
      new Promise<ServerSample[]>((res) => {
        resolveHistory = res
      }),
    )
    const { unmount } = renderHook(() => useHistory('server', 0, 1000))
    unmount()

    // Act & Assert — the unmount guard swallows the late resolution
    await act(async () => {
      resolveHistory([])
    })
  })
})
