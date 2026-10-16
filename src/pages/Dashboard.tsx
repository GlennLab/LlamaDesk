// Dashboard — live overview of connection, models, server metrics and GPUs.
import { ConnectionBadge } from '../components/layout'
import { MetricGrid } from '../components/cards'
import { Badge, EmptyState, ErrorBanner, Loading, MetricRow, Panel, StatCard } from '../components/ui'
import { useSnapshotContext } from '../hooks/useMonitor'
import {
  PLACEHOLDER,
  formatMiB,
  formatPercent,
  formatRate,
  formatTimestamp,
} from '../types'

function Dashboard() {
  const { snapshot, loading, error } = useSnapshotContext()

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
      </MetricGrid>

      <Panel title="GPU memory" subtitle="VRAM used / total per GPU">
        {snapshot.gpus.length > 0 ? (
          <div className="stack">
            {snapshot.gpus.map((gpu) => (
              <MetricRow
                key={gpu.index}
                label={`GPU ${gpu.index}`}
                value={`${formatMiB(gpu.memoryUsedMib)} / ${formatMiB(gpu.memoryTotalMib)}`}
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
    </div>
  )
}

export default Dashboard
