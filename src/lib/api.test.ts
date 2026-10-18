import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { AppConfig, MonitorSnapshot } from '../types'

// Hoisted so the vi.mock factories below can reference them.
const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }))

import {
  describeError,
  getGpuHistory,
  getServerHistory,
  getSettings,
  getSnapshot,
  pruneHistory,
  subscribeSnapshot,
  testConnection,
  updateSettings,
} from './api'

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

function snapshot(): MonitorSnapshot {
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
  }
}

beforeEach(() => {
  invokeMock.mockReset()
  listenMock.mockReset()
})

describe('command wrappers', () => {
  it('should invoke get_snapshot with no arguments', async () => {
    // Arrange
    const snap = snapshot()
    invokeMock.mockResolvedValue(snap)

    // Act
    const result = await getSnapshot()

    // Assert
    expect(invokeMock).toHaveBeenCalledTimes(1)
    expect(invokeMock).toHaveBeenCalledWith('get_snapshot')
    expect(result).toBe(snap)
  })

  it('should invoke get_settings with no arguments', async () => {
    // Arrange
    invokeMock.mockResolvedValue(CONFIG)

    // Act
    const result = await getSettings()

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('get_settings')
    expect(result).toBe(CONFIG)
  })

  it('should invoke update_settings with the config under the "config" key', async () => {
    // Arrange
    const saved: AppConfig = { ...CONFIG, serverIntervalMs: 3000 }
    invokeMock.mockResolvedValue(saved)

    // Act
    const result = await updateSettings(CONFIG)

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('update_settings', { config: CONFIG })
    expect(result).toBe(saved)
  })

  it('should invoke test_connection with an undefined url when none is given', async () => {
    // Arrange
    invokeMock.mockResolvedValue({ ok: true, message: 'ok' })

    // Act
    await testConnection()

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('test_connection', { url: undefined })
  })

  it('should invoke test_connection with the supplied url override', async () => {
    // Arrange
    invokeMock.mockResolvedValue({ ok: true, message: 'ok' })

    // Act
    await testConnection('http://example.test:9000')

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('test_connection', {
      url: 'http://example.test:9000',
    })
  })

  it('should invoke get_server_history with the millisecond range', async () => {
    // Arrange
    invokeMock.mockResolvedValue([])

    // Act
    await getServerHistory(1000, 2000)

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('get_server_history', {
      fromMs: 1000,
      toMs: 2000,
    })
  })

  it('should invoke get_gpu_history with the millisecond range', async () => {
    // Arrange
    invokeMock.mockResolvedValue([])

    // Act
    await getGpuHistory(1000, 2000)

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('get_gpu_history', {
      fromMs: 1000,
      toMs: 2000,
    })
  })

  it('should invoke prune_history with no arguments and return the row count', async () => {
    // Arrange
    invokeMock.mockResolvedValue(42)

    // Act
    const removed = await pruneHistory()

    // Assert
    expect(invokeMock).toHaveBeenCalledWith('prune_history')
    expect(removed).toBe(42)
  })

  it('should propagate a rejected command error unchanged', async () => {
    // Arrange
    invokeMock.mockRejectedValue('database is locked')

    // Act & Assert
    await expect(getSnapshot()).rejects.toBe('database is locked')
  })

  it('should propagate a rejected Error instance to the caller', async () => {
    // Arrange
    const failure = new Error('ipc closed')
    invokeMock.mockRejectedValue(failure)

    // Act & Assert
    await expect(updateSettings(CONFIG)).rejects.toBe(failure)
  })
})

describe('subscribeSnapshot', () => {
  it('should register a monitor:snapshot listener and forward the payload', async () => {
    // Arrange
    const unlisten = vi.fn()
    listenMock.mockResolvedValue(unlisten)
    const cb = vi.fn()
    const snap = snapshot()

    // Act
    const handle = await subscribeSnapshot(cb)

    // Assert
    expect(listenMock).toHaveBeenCalledTimes(1)
    expect(listenMock.mock.calls[0][0]).toBe('monitor:snapshot')
    const handler = listenMock.mock.calls[0][1] as (e: { payload: MonitorSnapshot }) => void
    expect(typeof handler).toBe('function')

    handler({ payload: snap })
    expect(cb).toHaveBeenCalledWith(snap)
    expect(handle).toBe(unlisten)
  })
})

describe('describeError', () => {
  it('should return a string rejection as-is', () => {
    expect(describeError('boom')).toBe('boom')
  })

  it('should return the message of an Error instance', () => {
    expect(describeError(new Error('kaboom'))).toBe('kaboom')
  })

  it('should stringify a non-Error value', () => {
    expect(describeError(42)).toBe('42')
    expect(describeError(null)).toBe('null')
    expect(describeError(undefined)).toBe('undefined')
    expect(describeError({ code: 1 })).toBe('[object Object]')
  })
})
