// Hardware — per-GPU detail plus a totals summary computed from non-null values.
import { GpuCard, MetricGrid } from '../components/cards'
import { EmptyState, ErrorBanner, Loading, StatCard } from '../components/ui'
import { useSnapshotContext } from '../hooks/useMonitor'
import { formatMiB, formatPercent } from '../types'

function Hardware() {
  const { snapshot, loading, error } = useSnapshotContext()

  if (loading && snapshot === null) return <Loading label="Loading GPU data…" />
  if (snapshot === null) {
    return error ? (
      <ErrorBanner message={error} />
    ) : (
      <EmptyState title="No data yet" message="Waiting for the first monitor snapshot." />
    )
  }

  const gpus = snapshot.gpus
  if (!snapshot.gpuAvailable || gpus.length === 0) {
    return (
      <EmptyState
        title="GPU monitoring unavailable"
        message="NVIDIA monitoring is unavailable (gpuAvailable is false or no GPUs were detected)."
      />
    )
  }

  let usedVram = 0
  let totalVram = 0
  let usedSeen = false
  let totalSeen = false
  let utilSum = 0
  let utilCount = 0
  for (const gpu of gpus) {
    if (gpu.memoryUsedMib !== null) {
      usedVram += gpu.memoryUsedMib
      usedSeen = true
    }
    if (gpu.memoryTotalMib !== null) {
      totalVram += gpu.memoryTotalMib
      totalSeen = true
    }
    if (gpu.utilizationPercent !== null) {
      utilSum += gpu.utilizationPercent
      utilCount += 1
    }
  }
  const totalUsed = usedSeen ? usedVram : null
  const totalMem = totalSeen ? totalVram : null
  const memPercent = totalUsed !== null && totalMem !== null && totalMem > 0 ? (totalUsed / totalMem) * 100 : null
  const avgUtil = utilCount > 0 ? utilSum / utilCount : null

  return (
    <div className="stack">
      {snapshot.errors.length > 0 ? (
        <div className="stack">
          {snapshot.errors.map((e) => (
            <ErrorBanner key={e.slice} message={`${e.slice}: ${e.message}`} />
          ))}
        </div>
      ) : null}

      <MetricGrid>
        <StatCard label="GPUs" value={gpus.length} />
        <StatCard label="Total VRAM" value={formatMiB(totalMem)} />
        <StatCard
          label="VRAM used"
          value={formatMiB(totalUsed)}
          hint={memPercent !== null ? formatPercent(memPercent) : undefined}
        />
        <StatCard
          label="Average utilization"
          value={formatPercent(avgUtil)}
          hint={`over ${utilCount} of ${gpus.length} GPUs`}
        />
      </MetricGrid>

      <MetricGrid>
        {gpus.map((gpu) => (
          <GpuCard key={gpu.index} gpu={gpu} />
        ))}
      </MetricGrid>
    </div>
  )
}

export default Hardware
