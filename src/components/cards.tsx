// Domain cards composed from the shared UI primitives: slots, GPUs, models.
// Every nullable field renders the em-dash placeholder — never a coerced 0.
import type { ReactNode } from 'react'
import { Badge, MetricRow, Nullable, Panel } from './ui'
import type { Tone } from './ui'
import {
  PLACEHOLDER,
  formatMiB,
  formatPercent,
} from '../types'
import type { GpuStats, ModelInfo, ModelStatus, Slot } from '../types'

const WARN_TEMPERATURE_C = 80

/** A slim horizontal progress bar (0–100 %). */
function Bar({ percent, color = '#4c9aff' }: { percent: number; color?: string }) {
  const width = Math.max(0, Math.min(100, percent))
  return (
    <div
      style={{
        height: 6,
        marginTop: 6,
        borderRadius: 3,
        background: '#232a33',
        overflow: 'hidden',
      }}
    >
      <div style={{ height: '100%', width: `${width}%`, background: color }} />
    </div>
  )
}

/** Formats a raw count, or the placeholder when null. */
function count(v: number | null): string {
  return v === null ? PLACEHOLDER : v.toLocaleString()
}

/** Formats "tokens remaining"; a negative value (idle slots use -1) means unlimited. */
function tokensRemaining(v: number | null): string {
  return v !== null && v < 0 ? 'unlimited' : count(v)
}

// ---------------------------------------------------------------------------
// MetricGrid
// ---------------------------------------------------------------------------

/** Props for {@link MetricGrid}. */
export interface MetricGridProps {
  children: ReactNode
}

/** A responsive, children-based grid for stat cards and compact metrics. */
export function MetricGrid({ children }: MetricGridProps) {
  return (
    <div
      style={{
        display: 'grid',
        gap: 'var(--gap)',
        gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))',
      }}
    >
      {children}
    </div>
  )
}

// ---------------------------------------------------------------------------
// SlotCard
// ---------------------------------------------------------------------------

/** Props for {@link SlotCard}. */
export interface SlotCardProps {
  slot: Slot
  index: number
}

/** A single server slot: state, task, prompt progress and next-token counters. */
export function SlotCard({ slot, index }: SlotCardProps) {
  const nt = slot.nextToken[0] ?? null
  const progressPercent =
    slot.nPromptTokens !== null && slot.nPromptTokensProcessed !== null && slot.nPromptTokens > 0
      ? (slot.nPromptTokensProcessed / slot.nPromptTokens) * 100
      : null

  return (
    <Panel
      title={`Slot ${slot.id}`}
      subtitle={`#${index}`}
      actions={
        <Badge tone={slot.isProcessing ? 'accent' : 'default'}>
          {slot.isProcessing ? 'processing' : 'idle'}
        </Badge>
      }
    >
      <MetricRow label="Task id" value={<Nullable value={slot.idTask} render={(v) => v} />} />
      <MetricRow label="Context size" value={<Nullable value={slot.nCtx} render={count} />} />
      <MetricRow
        label="Prompt progress"
        value={
          progressPercent === null ? (
            <span className="muted">{PLACEHOLDER}</span>
          ) : (
            <span>
              {formatPercent(progressPercent)}
              <Bar percent={progressPercent} />
            </span>
          )
        }
      />
      <MetricRow label="Prompt tokens" value={count(slot.nPromptTokens)} />
      <MetricRow label="Prompt processed" value={count(slot.nPromptTokensProcessed)} />
      <MetricRow label="Prompt cache" value={count(slot.nPromptTokensCache)} />
      <MetricRow label="Tokens remaining" value={<Nullable value={nt?.nRemain ?? null} render={tokensRemaining} />} />
      <MetricRow label="Tokens decoded" value={<Nullable value={nt?.nDecoded ?? null} render={count} />} />
      <MetricRow
        label="Has next token"
        value={<Nullable value={nt?.hasNextToken ?? null} render={(v) => (v ? 'yes' : 'no')} />}
      />
      <MetricRow
        label="Has new line"
        value={<Nullable value={nt?.hasNewLine ?? null} render={(v) => (v ? 'yes' : 'no')} />}
      />
    </Panel>
  )
}

// ---------------------------------------------------------------------------
// GpuCard
// ---------------------------------------------------------------------------

/** Props for {@link GpuCard}. */
export interface GpuCardProps {
  gpu: GpuStats
}

/** A GPU panel: utilization, VRAM usage, temperature and power. */
export function GpuCard({ gpu }: GpuCardProps) {
  const vramPercent =
    gpu.memoryUsedMib !== null && gpu.memoryTotalMib !== null && gpu.memoryTotalMib > 0
      ? (gpu.memoryUsedMib / gpu.memoryTotalMib) * 100
      : null
  const tempTone: Tone = gpu.temperatureC !== null && gpu.temperatureC >= WARN_TEMPERATURE_C ? 'warn' : 'default'

  return (
    <Panel
      title={`GPU ${gpu.index}`}
      subtitle={gpu.name}
      actions={<Badge tone="default">{formatPercent(gpu.utilizationPercent)}</Badge>}
    >
      <MetricRow
        label="Utilization"
        value={
          <span>
            {formatPercent(gpu.utilizationPercent)}
            {gpu.utilizationPercent !== null ? <Bar percent={gpu.utilizationPercent} /> : null}
          </span>
        }
      />
      <MetricRow
        label="VRAM"
        value={
          <span>
            {formatMiB(gpu.memoryUsedMib)} / {formatMiB(gpu.memoryTotalMib)}
            {vramPercent !== null ? ` (${formatPercent(vramPercent)})` : null}
            {vramPercent !== null ? <Bar percent={vramPercent} color="#3fb950" /> : null}
          </span>
        }
      />
      <MetricRow
        label="Temperature"
        value={<Badge tone={tempTone}>{gpu.temperatureC === null ? PLACEHOLDER : `${gpu.temperatureC.toFixed(1)}°C`}</Badge>}
      />
      <MetricRow
        label="Power"
        value={
          gpu.powerDrawW === null
            ? PLACEHOLDER
            : gpu.powerLimitW === null
              ? `${gpu.powerDrawW.toFixed(0)} W`
              : `${gpu.powerDrawW.toFixed(0)} / ${gpu.powerLimitW.toFixed(0)} W`
        }
      />
    </Panel>
  )
}

// ---------------------------------------------------------------------------
// ModelRow
// ---------------------------------------------------------------------------

/** Maps a `ModelStatus` (string or `{unknown}`) to a badge tone. */
function statusTone(status: ModelStatus): Tone {
  if (status === 'loaded') return 'ok'
  if (status === 'loading') return 'warn'
  if (status === 'unloaded') return 'default'
  return 'danger'
}

/** Maps a `ModelStatus` (string or `{unknown}`) to a display label. */
function statusLabel(status: ModelStatus): string {
  if (typeof status === 'string') return status
  return `unknown: ${status.unknown}`
}

/** Props for {@link ModelRow}. */
export interface ModelRowProps {
  model: ModelInfo
}

/** A single model line: id, status, context length, source and removability. */
export function ModelRow({ model }: ModelRowProps) {
  return (
    <div className="metric-row">
      <span className="metric-label num" style={{ color: 'var(--text)' }}>
        {model.id}
      </span>
      <span className="row" style={{ justifyContent: 'flex-end', gap: 10 }}>
        <Badge tone={statusTone(model.status)}>{statusLabel(model.status)}</Badge>
        <span className="num">{model.contextLength === null ? PLACEHOLDER : `${model.contextLength.toLocaleString()} ctx`}</span>
        <span className="muted">{model.source ?? PLACEHOLDER}</span>
        <span className="muted">{model.canRemove ? 'removable' : 'locked'}</span>
      </span>
    </div>
  )
}
