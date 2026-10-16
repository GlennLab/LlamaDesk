// Slots — per loaded model, its slots with live processing state.
import { SlotCard } from '../components/cards'
import { EmptyState, ErrorBanner, Loading, Panel } from '../components/ui'
import { useSnapshotContext } from '../hooks/useMonitor'
import { PLACEHOLDER } from '../types'

function Slots() {
  const { snapshot, loading, error } = useSnapshotContext()

  if (loading && snapshot === null) return <Loading label="Loading slots…" />
  if (snapshot === null) {
    return error ? (
      <ErrorBanner message={error} />
    ) : (
      <EmptyState title="No data yet" message="Waiting for the first monitor snapshot." />
    )
  }

  if (snapshot.loadedModels.length === 0) {
    return (
      <EmptyState
        title="No model loaded"
        message="Slots are reported per loaded model. Load a model in llama.cpp to see them."
      />
    )
  }

  return (
    <div className="stack">
      {snapshot.errors.length > 0 ? (
        <div className="stack">
          {snapshot.errors.map((e) => (
            <ErrorBanner key={e.slice} message={`${e.slice}: ${e.message}`} />
          ))}
        </div>
      ) : null}

      {snapshot.loadedModels.map((id) => {
        const slots = snapshot.slots[id] ?? []
        const metrics = snapshot.metrics[id] ?? null
        return (
          <Panel
            key={id}
            title={id}
            subtitle={`active ${metrics?.activeRequests ?? PLACEHOLDER} · deferred ${metrics?.deferredRequests ?? PLACEHOLDER}`}
          >
            {slots.length === 0 ? (
              <EmptyState title="No slots" message={`No slots reported for ${id}.`} />
            ) : (
              <div className="grid cols-2">
                {slots.map((slot, i) => (
                  <SlotCard key={slot.id} slot={slot} index={i} />
                ))}
              </div>
            )}
          </Panel>
        )
      })}
    </div>
  )
}

export default Slots
