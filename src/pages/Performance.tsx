// Performance — persisted server & GPU history with a time-range selector.
import { useState } from 'react'
import { TimelineChart } from '../components/charts'
import type { ChartSeries } from '../components/charts'
import { EmptyState, ErrorBanner, Loading, Panel } from '../components/ui'
import { useHistory } from '../hooks/useMonitor'
import { formatMiB, formatRate, formatTemperature, type GpuSample } from '../types'

const MINUTE_MS = 60_000
const DEFAULT_RANGE_MS = 60 * MINUTE_MS

/** Selectable history windows. */
const RANGES: ReadonlyArray<{ label: string; ms: number }> = [
  { label: 'Last 15 min', ms: 15 * MINUTE_MS },
  { label: 'Last 1 hour', ms: 60 * MINUTE_MS },
  { label: 'Last 6 hours', ms: 360 * MINUTE_MS },
  { label: 'Last 24 hours', ms: 1440 * MINUTE_MS },
]

/** Distinct series colours, cycled per GPU index. */
const SERIES_COLORS = ['#4c9aff', '#3fb950', '#d29922', '#f85149', '#a371f7', '#39c5cf']

function Performance() {
  const [rangeMs, setRangeMs] = useState(DEFAULT_RANGE_MS)
  const [win, setWin] = useState(() => ({ from: Date.now() - DEFAULT_RANGE_MS, to: Date.now() }))

  const server = useHistory('server', win.from, win.to)
  const gpu = useHistory('gpu', win.from, win.to)

  function selectRange(ms: number) {
    setRangeMs(ms)
    setWin({ from: Date.now() - ms, to: Date.now() })
  }

  function refresh() {
    server.reload()
    gpu.reload()
  }

  const rangeLabel = RANGES.find((r) => r.ms === rangeMs)?.label ?? 'custom'
  const loading = server.loading || gpu.loading
  const empty = server.samples.length === 0 && gpu.samples.length === 0

  const promptSeries: ChartSeries[] = [
    {
      name: 'prompt tok/s',
      color: SERIES_COLORS[0],
      type: 'area',
      data: server.samples.map((s) => [s.timestampMs, s.promptTokensPerSec] as [number, number | null]),
    },
  ]
  const generationSeries: ChartSeries[] = [
    {
      name: 'generation tok/s',
      color: SERIES_COLORS[1],
      type: 'area',
      data: server.samples.map((s) => [s.timestampMs, s.generationTokensPerSec] as [number, number | null]),
    },
  ]
  const activeSeries: ChartSeries[] = [
    {
      name: 'active requests',
      color: SERIES_COLORS[2],
      data: server.samples.map((s) => [s.timestampMs, s.activeRequests] as [number, number | null]),
    },
  ]
  const deferredSeries: ChartSeries[] = [
    {
      name: 'deferred requests',
      color: SERIES_COLORS[3],
      data: server.samples.map((s) => [s.timestampMs, s.deferredRequests] as [number, number | null]),
    },
  ]

  const gpuIndices = Array.from(new Set(gpu.samples.map((s) => s.gpuIndex))).sort((a, b) => a - b)

  /** GPU samples grouped by timestamp so cross-card aggregates can be computed per tick. */
  const gpuByTs = new Map<number, GpuSample[]>()
  for (const s of gpu.samples) {
    const bucket = gpuByTs.get(s.timestampMs) ?? []
    bucket.push(s)
    gpuByTs.set(s.timestampMs, bucket)
  }
  const sortedTs = [...gpuByTs.keys()].sort((a, b) => a - b)

  const utilizationSeries: ChartSeries[] = gpuIndices.map((idx, i) => ({
    name: `GPU ${idx}`,
    color: SERIES_COLORS[i % SERIES_COLORS.length],
    data: gpu.samples
      .filter((s) => s.gpuIndex === idx)
      .map((s) => [s.timestampMs, s.utilizationPercent] as [number, number | null]),
  }))
  if (gpu.samples.length > 0) {
    utilizationSeries.push({
      name: 'All GPUs (avg)',
      color: '#9ca5b0',
      data: sortedTs.map((ts) => {
        const values = gpuByTs.get(ts)!
          .map((s) => s.utilizationPercent)
          .filter((v): v is number => v !== null)
        return [
          ts,
          values.length > 0 ? Math.round(values.reduce((a, b) => a + b, 0) / values.length) : null,
        ]
      }),
    })
  }

  const vramSeries: ChartSeries[] = gpuIndices.map((idx, i) => ({
    name: `GPU ${idx}`,
    color: SERIES_COLORS[i % SERIES_COLORS.length],
    data: gpu.samples
      .filter((s) => s.gpuIndex === idx)
      .map((s) => [s.timestampMs, s.memoryUsedMib] as [number, number | null]),
  }))
  if (gpu.samples.length > 0) {
    vramSeries.push({
      name: 'Total',
      color: '#9ca5b0',
      data: sortedTs.map((ts) => {
        const values = gpuByTs.get(ts)!
          .map((s) => s.memoryUsedMib)
          .filter((v): v is number => v !== null)
        return [ts, values.length > 0 ? values.reduce((a, b) => a + b, 0) : null]
      }),
    })
  }

  const temperatureSeries: ChartSeries[] = gpuIndices.map((idx, i) => ({
    name: `GPU ${idx}`,
    color: SERIES_COLORS[i % SERIES_COLORS.length],
    data: gpu.samples
      .filter((s) => s.gpuIndex === idx)
      .map((s) => [s.timestampMs, s.temperatureC] as [number, number | null]),
  }))

  const fanSeries: ChartSeries[] = gpuIndices.map((idx, i) => ({
    name: `GPU ${idx}`,
    color: SERIES_COLORS[i % SERIES_COLORS.length],
    data: gpu.samples
      .filter((s) => s.gpuIndex === idx)
      .map((s) => [s.timestampMs, s.fanSpeedPercent] as [number, number | null]),
  }))
  if (gpu.samples.length > 0) {
    fanSeries.push({
      name: 'All GPUs (avg)',
      color: '#9ca5b0',
      data: sortedTs.map((ts) => {
        const values = gpuByTs.get(ts)!
          .map((s) => s.fanSpeedPercent)
          .filter((v): v is number => v !== null)
        return [
          ts,
          values.length > 0 ? Math.round(values.reduce((a, b) => a + b, 0) / values.length) : null,
        ]
      }),
    })
  }

  return (
    <div className="stack">
      <Panel
        title="Performance history"
        subtitle={`Window: ${rangeLabel}`}
        actions={
          <>
            <select
              aria-label="Time range"
              value={rangeMs}
              onChange={(e) => selectRange(Number(e.target.value))}
              style={{ width: 160 }}
            >
              {RANGES.map((r) => (
                <option key={r.ms} value={r.ms}>
                  {r.label}
                </option>
              ))}
            </select>
            <button type="button" className="btn" onClick={refresh} disabled={loading}>
              Refresh
            </button>
          </>
        }
      >
        <div className="stack">
          {server.error ? <ErrorBanner message={server.error} /> : null}
          {gpu.error ? <ErrorBanner message={gpu.error} /> : null}

          {loading && empty ? <Loading label="Loading history…" /> : null}

          {!loading && empty ? (
            <EmptyState
              title="No history yet"
              message="Samples are persisted while monitoring runs. Data will appear here shortly."
            />
          ) : null}

          {server.samples.length > 0 ? (
            <div className="grid cols-2">
              <TimelineChart
                title="Prompt processing"
                series={promptSeries}
                yAxisName="tok/s"
                valueFormatter={(v) => formatRate(v, '')}
              />
              <TimelineChart
                title="Generation speed"
                series={generationSeries}
                yAxisName="tok/s"
                valueFormatter={(v) => formatRate(v, '')}
              />
              <TimelineChart
                title="Active requests"
                series={activeSeries}
                valueFormatter={(v) => v.toFixed(0)}
                integerTicks
              />
              <TimelineChart
                title="Deferred requests"
                series={deferredSeries}
                valueFormatter={(v) => v.toFixed(0)}
                integerTicks
              />
            </div>
          ) : null}

          {gpu.samples.length > 0 ? (
            <div className="grid cols-2">
              <TimelineChart
                title="GPU utilization"
                series={utilizationSeries}
                yAxisName="%"
                valueFormatter={(v) => `${v.toFixed(0)}%`}
              />
              <TimelineChart
                title="GPU VRAM used"
                series={vramSeries}
                yAxisName="GiB"
                valueFormatter={(v) => formatMiB(v)}
              />
              <TimelineChart
                title="GPU fan speed"
                series={fanSeries}
                yAxisName="%"
                valueFormatter={(v) => `${v.toFixed(0)}%`}
              />
              <TimelineChart
                title="GPU temperature"
                series={temperatureSeries}
                yAxisName="°C"
                valueFormatter={(v) => formatTemperature(v)}
              />
            </div>
          ) : null}
        </div>
      </Panel>
    </div>
  )
}

export default Performance
