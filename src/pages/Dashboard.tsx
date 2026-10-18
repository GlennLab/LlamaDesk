// Dashboard — live overview of connection, models, server metrics and GPUs.
import { useState } from 'react'
import { ConnectionBadge } from '../components/layout'
import { MetricGrid } from '../components/cards'
import { Badge, EmptyState, ErrorBanner, Loading, MetricRow, Panel, StatCard } from '../components/ui'
import { useHistory, useSnapshotContext } from '../hooks/useMonitor'
import { describeError, updateLlama } from '../lib/api'
import {
  PLACEHOLDER,
  UpdateResult,
  formatMiB,
  formatPercent,
  formatPower,
  formatRate,
  formatTimestamp,
} from '../types'

function UpdatePanel() {
  const [state, setState] = useState<{
    status: 'idle' | 'running' | 'done' | 'error'
    result?: UpdateResult
  }>({ status: 'idle' })

  async function onUpdate() {
    setState({ status: 'running' })
    try {
      const result = await updateLlama()
      setState({ status: 'done', result })
    } catch (e) {
      setState({
        status: 'error',
        result: { success: false, oldVersion: null, newVersion: null, output: describeError(e) },
      })
    }
  }

  const showResult = state.status === 'done' || state.status === 'error'

  return (
    <Panel title="llama.cpp update" subtitle="Update the server binary via git pull + rebuild">
      <button type="button" disabled={state.status === 'running'} onClick={onUpdate}>
        {state.status === 'running' ? 'Updating…' : 'Update llama.cpp'}
      </button>
      {showResult && state.result && (
        <>
          <MetricRow label="Before" value={state.result.oldVersion ?? PLACEHOLDER} />
          <MetricRow label="After" value={state.result.newVersion ?? PLACEHOLDER} />
          {state.result.success ? (
            <Badge tone="ok">Update succeeded</Badge>
          ) : (
            <ErrorBanner message={`Update failed: ${state.result.output}`} />
          )}
          <details>
            <summary>Output</summary>
            <pre>{state.result.output}</pre>
          </details>
        </>
      )}
    </Panel>
  )
}

function Dashboard() {
  const { snapshot, loading, error } = useSnapshotContext()
  // Stabilize the history window once on mount so `useHistory` does not
  // refetch on every render (Date.now() changes each render).
  const [historyWindow] = useState(() => [
    Date.now() - 5 * 60_000,
    Date.now(),
  ])
  const history = useHistory('server', historyWindow[0], historyWindow[1])
  const rates = history.samples
    .map((s) => s.generationTokensPerSec)
    .filter((v): v is number => v !== null)
  const smoothedRate = rates.length
    ? rates.reduce((a, b) => a + b, 0) / rates.length
    : null

  if (loading && snapshot === null) return <Loading label="Loading snapshot…" />
  if (snapshot === null) {
    return error ? (
      <ErrorBanner message={error} />
    ) : (
      <EmptyState title="No data yet" message="Waiting for the first monitor snapshot." />
    )
  }

  const primaryId = snapshot.loadedModels[0] ?? null
  const primaryMetrics = primaryId !== null ? snapshot.metrics[primaryId] ?? null : null
  const primarySlots = primaryId !== null ? snapshot.slots[primaryId] ?? [] : []
  const busySlots = primarySlots.filter((s) => s.isProcessing).length
  const freeSlots = primaryId !== null ? primarySlots.length - busySlots : null

  let usedVram = 0
  let totalVram = 0
  let usedSeen = false
  let totalSeen = false
  for (const gpu of snapshot.gpus) {
    if (gpu.memoryUsedMib !== null) {
      usedVram += gpu.memoryUsedMib
      usedSeen = true
    }
    if (gpu.memoryTotalMib !== null) {
      totalVram += gpu.memoryTotalMib
      totalSeen = true
    }
  }
  const overallVramPercent = usedSeen && totalSeen && totalVram > 0 ? (usedVram / totalVram) * 100 : null

  return (
    <div className="stack">
      {snapshot.errors.length > 0 ? (
        <div className="stack">
          {snapshot.errors.map((e) => (
            <ErrorBanner key={e.slice} message={`${e.slice}: ${e.message}`} />
          ))}
        </div>
      ) : null}

      <div className="grid cols-2">
        <Panel title="Connection">
          <MetricRow label="Status" value={<ConnectionBadge state={snapshot.connection} />} />
          <MetricRow label="Server URL" value={snapshot.serverUrl} />
          <MetricRow label="Last updated" value={formatTimestamp(snapshot.lastUpdated)} />
        </Panel>

        <Panel title="Server">
          <MetricRow label="Role" value={snapshot.props?.role ?? PLACEHOLDER} />
          <MetricRow label="Build" value={snapshot.props?.buildInfo ?? PLACEHOLDER} />
          <MetricRow label="Models" value={snapshot.models.length} />
          <MetricRow
            label="Context length"
            value={snapshot.models.find((m) => m.id === primaryId)?.contextLength ?? PLACEHOLDER}
          />
        </Panel>
      </div>

      <Panel
        title="Active models"
        subtitle={primaryId !== null ? 'Metrics below use the first loaded model' : undefined}
      >
        {snapshot.loadedModels.length > 0 ? (
          <div className="row" style={{ flexWrap: 'wrap' }}>
            {snapshot.loadedModels.map((id) => (
              <Badge key={id} tone={id === primaryId ? 'accent' : 'default'}>
                {id}
              </Badge>
            ))}
          </div>
        ) : (
          <EmptyState title="No model loaded" message="Load a model in llama.cpp to populate metrics." />
        )}
      </Panel>

      <Panel title="Decode slots">
        {primarySlots.length > 0 ? (
          primarySlots.map((slot) => (
            <MetricRow
              key={slot.id}
              label={`Slot ${slot.id}`}
              value={`${slot.nCtx ?? '—'} ctx · ${slot.nextToken[0]?.nRemain ?? '—'} rem · ${slot.nextToken[0]?.nDecoded ?? '—'} dec${slot.isProcessing ? ' · processing' : ' · idle'}`}
            />
          ))
        ) : (
          <EmptyState title="No slots" message="Load a model to see decode slots." />
        )}
      </Panel>

      <MetricGrid>
        <StatCard
          label="Active requests"
          value={primaryMetrics?.activeRequests ?? PLACEHOLDER}
          hint={primaryId ?? 'no model loaded'}
        />
        <StatCard
          label="Free slots"
          value={freeSlots ?? PLACEHOLDER}
          hint={primaryId !== null ? 'derived: total − processing' : 'no model loaded'}
        />
        <StatCard
          label="Deferred requests"
          value={primaryMetrics?.deferredRequests ?? PLACEHOLDER}
          hint={primaryId ?? 'no model loaded'}
        />
        <StatCard
          label="Prompt processing"
          value={formatRate(primaryMetrics?.promptTokensPerSec ?? null, '')}
          unit="tok/s"
        />
        <StatCard
          label="Generation speed"
          value={formatRate(primaryMetrics?.generationTokensPerSec ?? null, '')}
          unit="tok/s"
        />
        <StatCard
          label="KV cache usage"
          value={primaryMetrics !== null && primaryMetrics.kvCacheUsageRatio !== null ? formatPercent(primaryMetrics.kvCacheUsageRatio * 100) : PLACEHOLDER}
          hint={primaryId ?? 'no model loaded'}
        />
        <StatCard
          label="Tokens predicted (total)"
          value={primaryMetrics?.tokensPredictedTotal ?? PLACEHOLDER}
          hint="cumulatief, alleen oplopend"
        />
        <StatCard
          label="Generation speed (avg 5m)"
          value={formatRate(smoothedRate, '')}
          unit="tok/s"
          hint={primaryId ?? 'no model loaded'}
        />
        <StatCard
          label="Tokens decoded (total)"
          value={primaryMetrics?.nDecodeTotal ?? PLACEHOLDER}
          hint="cumulatief, alleen oplopend"
        />
      </MetricGrid>

      <Panel title="GPU memory" subtitle="VRAM used / total per GPU">
        {snapshot.gpus.length > 0 ? (
          <div className="stack">
            {snapshot.gpus.map((gpu) => (
              <MetricRow
                key={gpu.index}
                label={`GPU ${gpu.index}`}
                value={`${formatMiB(gpu.memoryUsedMib)} / ${formatMiB(gpu.memoryTotalMib)} · ${formatPower(gpu.powerDrawW, gpu.powerLimitW)}`}
              />
            ))}
            <MetricRow
              label="Overall"
              value={`${formatMiB(usedSeen ? usedVram : null)} / ${formatMiB(totalSeen ? totalVram : null)}${overallVramPercent !== null ? ` (${formatPercent(overallVramPercent)})` : ''}`}
            />
          </div>
        ) : (
          <EmptyState
            title="No GPU data"
            message={
              snapshot.gpuAvailable
                ? 'No GPUs were detected.'
                : 'NVIDIA monitoring is unavailable.'
            }
          />
        )}
      </Panel>

      <UpdatePanel />
    </div>
  )
}

export default Dashboard
