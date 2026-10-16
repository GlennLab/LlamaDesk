// Deterministic domain fixtures shared by the component and page tests.
// No backend, no network, no coercion: every missing value is an explicit `null`
// so the tests can pin the "null renders as a placeholder" contract end to end.
import type {
  GpuSample,
  GpuStats,
  ModelInfo,
  MonitorSnapshot,
  NextToken,
  ServerMetrics,
  ServerSample,
  Slot,
} from '../types'

/** Builds a `NextToken` with every field null unless overridden. */
export function makeNextToken(overrides: Partial<NextToken> = {}): NextToken {
  return {
    hasNextToken: null,
    hasNewLine: null,
    nRemain: null,
    nDecoded: null,
    ...overrides,
  }
}

/** Builds an idle `Slot` with no next-token counters unless overridden. */
export function makeSlot(overrides: Partial<Slot> = {}): Slot {
  return {
    id: 0,
    nCtx: null,
    speculative: null,
    isProcessing: false,
    idTask: null,
    nPromptTokens: null,
    nPromptTokensProcessed: null,
    nPromptTokensCache: null,
    nextToken: [],
    params: null,
    ...overrides,
  }
}

/** Builds an unloaded `ModelInfo` unless overridden. */
export function makeModel(overrides: Partial<ModelInfo> = {}): ModelInfo {
  return {
    id: 'model',
    aliases: [],
    tags: [],
    status: 'unloaded',
    statusArgs: [],
    source: null,
    canRemove: false,
    contextLength: null,
    architecture: null,
    ...overrides,
  }
}

/** Builds a `ServerMetrics` with every value null unless overridden. */
export function makeMetrics(overrides: Partial<ServerMetrics> = {}): ServerMetrics {
  return {
    promptTokensPerSec: null,
    generationTokensPerSec: null,
    activeRequests: null,
    deferredRequests: null,
    promptTokensTotal: null,
    promptTokensCachedTotal: null,
    tokensPredictedTotal: null,
    nDecodeTotal: null,
    nTokensMax: null,
    kvCacheUsageRatio: null,
    raw: {},
    ...overrides,
  }
}

/** Builds a `GpuStats` with every measurement null unless overridden. */
export function makeGpu(overrides: Partial<GpuStats> = {}): GpuStats {
  return {
    index: 0,
    name: 'GPU',
    utilizationPercent: null,
    memoryTotalMib: null,
    memoryUsedMib: null,
    temperatureC: null,
    powerDrawW: null,
    powerLimitW: null,
    ...overrides,
  }
}

/** Builds a `ServerSample` with every metric null unless overridden. */
export function makeServerSample(overrides: Partial<ServerSample> = {}): ServerSample {
  return {
    timestampMs: 0,
    promptTokensPerSec: null,
    generationTokensPerSec: null,
    activeRequests: null,
    deferredRequests: null,
    kvCacheUsageRatio: null,
    promptTokensTotal: null,
    tokensPredictedTotal: null,
    ...overrides,
  }
}

/** Builds a `GpuSample` with every metric null unless overridden. */
export function makeGpuSample(overrides: Partial<GpuSample> = {}): GpuSample {
  return {
    timestampMs: 0,
    gpuIndex: 0,
    utilizationPercent: null,
    memoryUsedMib: null,
    memoryTotalMib: null,
    temperatureC: null,
    powerDrawW: null,
    ...overrides,
  }
}

/** The baseline snapshot: connected, empty, all collections empty. */
export function makeSnapshot(overrides: Partial<MonitorSnapshot> = {}): MonitorSnapshot {
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

/** A disconnected, empty snapshot — the graceful-degradation case. */
export function emptySnapshot(): MonitorSnapshot {
  return makeSnapshot({ connection: 'disconnected' })
}

/**
 * A representative, fully-populated snapshot: one loaded model with three slots
 * (one processing, two idle using the `-1` "unlimited" sentinel) and two GPUs.
 * Used to assert pages render their key content, not just their empty states.
 */
export function representativeSnapshot(
  overrides: Partial<MonitorSnapshot> = {},
): MonitorSnapshot {
  const modelId = 'qwen-3-27b'
  return makeSnapshot({
    connection: 'connected',
    props: {
      role: 'router',
      maxInstances: 1,
      modelsAutoload: true,
      modelAlias: null,
      modelPath: null,
      buildInfo: 'b2794-62df1815',
      corsProxyEnabled: false,
    },
    models: [
      makeModel({
        id: modelId,
        status: 'loaded',
        contextLength: 32768,
        source: '/models/qwen.gguf',
        canRemove: true,
      }),
      makeModel({ id: 'llama-3-8b', status: 'unloaded' }),
    ],
    loadedModels: [modelId],
    slots: {
      [modelId]: [
        makeSlot({
          id: 0,
          nCtx: 4096,
          isProcessing: true,
          idTask: 42,
          nPromptTokens: 100,
          nPromptTokensProcessed: 100,
          nPromptTokensCache: 20,
          nextToken: [
            makeNextToken({ hasNextToken: true, hasNewLine: false, nRemain: 512, nDecoded: 12 }),
          ],
        }),
        makeSlot({
          id: 1,
          nCtx: 4096,
          isProcessing: false,
          nextToken: [
            makeNextToken({ hasNextToken: true, hasNewLine: false, nRemain: -1, nDecoded: 0 }),
          ],
        }),
        makeSlot({
          id: 2,
          nCtx: 4096,
          isProcessing: false,
          nextToken: [makeNextToken({ hasNextToken: false, nRemain: -1, nDecoded: 0 })],
        }),
      ],
    },
    metrics: {
      [modelId]: makeMetrics({
        activeRequests: 1,
        deferredRequests: 0,
        promptTokensPerSec: 123.4,
        generationTokensPerSec: 56.7,
      }),
    },
    gpus: [
      makeGpu({
        index: 0,
        name: 'RTX 4090',
        utilizationPercent: 50,
        memoryUsedMib: 1024,
        memoryTotalMib: 2048,
        temperatureC: 65,
        powerDrawW: 120,
        powerLimitW: 300,
      }),
      makeGpu({
        index: 1,
        name: 'RTX 4090',
        utilizationPercent: 25,
        memoryUsedMib: 512,
        memoryTotalMib: 2048,
        temperatureC: 40,
        powerDrawW: 60,
        powerLimitW: 300,
      }),
    ],
    gpuAvailable: true,
    lastUpdated: '2026-10-15T23:00:00.000Z',
    ...overrides,
  })
}
