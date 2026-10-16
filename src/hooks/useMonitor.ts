// React hooks over the typed Tauri API. All hooks return objects, not arrays.
import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'
import type { ReactNode } from 'react'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  describeError,
  getGpuHistory,
  getServerHistory,
  getSettings,
  getSnapshot,
  subscribeSnapshot,
  testConnection,
  updateSettings,
} from '../lib/api'
import type {
  AppConfig,
  ConnectionTest,
  GpuSample,
  MonitorSnapshot,
  ServerSample,
} from '../types'

/** State returned by {@link useSnapshot}. */
export interface SnapshotState {
  snapshot: MonitorSnapshot | null
  loading: boolean
  error: string | null
}

/**
 * Loads the current snapshot on mount and keeps it live via the
 * `monitor:snapshot` event. A stream error never clears the last good snapshot.
 */
export function useSnapshot(): SnapshotState {
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    let unlisten: UnlistenFn | null = null

    void getSnapshot()
      .then((next) => {
        if (!mounted.current) return
        setSnapshot(next)
        setError(null)
      })
      .catch((e) => {
        if (!mounted.current) return
        setError(describeError(e))
      })
      .finally(() => {
        if (mounted.current) setLoading(false)
      })

    subscribeSnapshot((next) => {
      if (!mounted.current) return
      setSnapshot(next)
      setError(null)
      setLoading(false)
    })
      .then((fn) => {
        if (mounted.current) unlisten = fn
        else fn()
      })
      .catch((e) => {
        if (mounted.current) setError(describeError(e))
      })

    return () => {
      mounted.current = false
      if (unlisten) unlisten()
    }
  }, [])

  return { snapshot, loading, error }
}

/** Shares the single live snapshot subscription with every descendant. */
const SnapshotContext = createContext<SnapshotState | null>(null)

/**
 * Registers the one `monitor:snapshot` subscription for the app and exposes it
 * to all consumers via {@link useSnapshotContext}. Mount this once, high in the
 * tree — never per page — so the listener and IPC load are not duplicated.
 */
export function SnapshotProvider({ children }: { children: ReactNode }): ReactNode {
  const value = useSnapshot()
  return createElement(SnapshotContext.Provider, { value }, children)
}

/**
 * Reads the snapshot shared by {@link SnapshotProvider}.
 *
 * @throws When called outside a `SnapshotProvider`.
 */
export function useSnapshotContext(): SnapshotState {
  const value = useContext(SnapshotContext)
  if (value === null) {
    throw new Error('useSnapshotContext must be used within a SnapshotProvider')
  }
  return value
}

/** State returned by {@link useSettings}. */
export interface SettingsState {
  config: AppConfig | null
  loading: boolean
  saving: boolean
  error: string | null
  /** Persists and applies the next config; rejects on failure. */
  save: (next: AppConfig) => Promise<void>
  /** Probes the server, optionally with a one-off URL override. */
  testUrl: (url?: string) => Promise<ConnectionTest>
}

/** Loads settings on mount and exposes save/test actions. */
export function useSettings(): SettingsState {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    void getSettings()
      .then((c) => {
        if (!mounted.current) return
        setConfig(c)
        setError(null)
      })
      .catch((e) => {
        if (mounted.current) setError(describeError(e))
      })
      .finally(() => {
        if (mounted.current) setLoading(false)
      })
    return () => {
      mounted.current = false
    }
  }, [])

  const save = useCallback(async (next: AppConfig): Promise<void> => {
    setSaving(true)
    try {
      const saved = await updateSettings(next)
      if (mounted.current) {
        setConfig(saved)
        setError(null)
      }
    } catch (e) {
      const message = describeError(e)
      if (mounted.current) setError(message)
      throw new Error(message)
    } finally {
      if (mounted.current) setSaving(false)
    }
  }, [])

  const testUrl = useCallback(
    (url?: string): Promise<ConnectionTest> => testConnection(url),
    [],
  )

  return { config, loading, saving, error, save, testUrl }
}

/** State returned by {@link useHistory}. */
export interface HistoryState<T> {
  samples: T[]
  loading: boolean
  error: string | null
  reload: () => void
}

/** Fetches persisted server-metric history for the given millisecond range. */
export function useHistory(
  kind: 'server',
  fromMs: number,
  toMs: number,
): HistoryState<ServerSample>
/** Fetches persisted GPU history for the given millisecond range. */
export function useHistory(
  kind: 'gpu',
  fromMs: number,
  toMs: number,
): HistoryState<GpuSample>
export function useHistory(
  kind: 'server' | 'gpu',
  fromMs: number,
  toMs: number,
): HistoryState<ServerSample | GpuSample> {
  const [samples, setSamples] = useState<ServerSample[] | GpuSample[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const mounted = useRef(true)

  const reload = useCallback((): void => {
    setLoading(true)
    void (async () => {
      try {
        const next: ServerSample[] | GpuSample[] =
          kind === 'server'
            ? await getServerHistory(fromMs, toMs)
            : await getGpuHistory(fromMs, toMs)
        if (!mounted.current) return
        setSamples(next)
        setError(null)
      } catch (e) {
        if (mounted.current) setError(describeError(e))
      } finally {
        if (mounted.current) setLoading(false)
      }
    })()
  }, [kind, fromMs, toMs])

  useEffect(() => {
    mounted.current = true
    reload()
    return () => {
      mounted.current = false
    }
  }, [reload])

  return { samples, loading, error, reload }
}
