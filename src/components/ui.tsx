// Reusable, typed presentational primitives. Plain DOM + CSS classes only.
import type { ReactNode } from 'react'
import { PLACEHOLDER } from '../types'

/** Colour tones shared by badges and stat cards. */
export type Tone = 'default' | 'ok' | 'warn' | 'danger' | 'accent'

// ---------------------------------------------------------------------------
// Panel
// ---------------------------------------------------------------------------

/** Props for {@link Panel}. */
export interface PanelProps {
  title: string
  subtitle?: string
  actions?: ReactNode
  children: ReactNode
}

/** A titled content container. */
export function Panel({ title, subtitle, actions, children }: PanelProps) {
  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2 className="panel-title">{title}</h2>
          {subtitle ? <p className="panel-subtitle">{subtitle}</p> : null}
        </div>
        {actions ? <div className="panel-actions">{actions}</div> : null}
      </header>
      <div className="panel-body">{children}</div>
    </section>
  )
}

// ---------------------------------------------------------------------------
// StatCard
// ---------------------------------------------------------------------------

/** Props for {@link StatCard}. */
export interface StatCardProps {
  label: string
  value: ReactNode
  unit?: string
  hint?: string
  tone?: Tone
}

/** A single highlighted metric. */
export function StatCard({ label, value, unit, hint, tone = 'default' }: StatCardProps) {
  return (
    <div className={`stat-card stat-${tone}`}>
      <div className="stat-label">{label}</div>
      <div className="stat-value">
        <span className="num">{value}</span>
        {unit ? <span className="stat-unit">{unit}</span> : null}
      </div>
      {hint ? <div className="stat-hint">{hint}</div> : null}
    </div>
  )
}

// ---------------------------------------------------------------------------
// MetricRow
// ---------------------------------------------------------------------------

/** Props for {@link MetricRow}. */
export interface MetricRowProps {
  label: string
  value: ReactNode
}

/** A label/value line, typically within a panel. */
export function MetricRow({ label, value }: MetricRowProps) {
  return (
    <div className="metric-row">
      <span className="metric-label">{label}</span>
      <span className="metric-value num">{value}</span>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

/** Props for {@link Badge}. */
export interface BadgeProps {
  tone?: Tone
  children: ReactNode
}

/** A small status pill. */
export function Badge({ tone = 'default', children }: BadgeProps) {
  return <span className={`badge badge-${tone}`}>{children}</span>
}

// ---------------------------------------------------------------------------
// EmptyState
// ---------------------------------------------------------------------------

/** Props for {@link EmptyState}. */
export interface EmptyStateProps {
  title: string
  message: string
}

/** Shown when there is nothing to display. */
export function EmptyState({ title, message }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <div className="empty-title">{title}</div>
      <div className="empty-message">{message}</div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// ErrorBanner
// ---------------------------------------------------------------------------

/** Props for {@link ErrorBanner}. */
export interface ErrorBannerProps {
  message: string
  onRetry?: () => void
}

/** Inline, non-fatal error with an optional retry action. */
export function ErrorBanner({ message, onRetry }: ErrorBannerProps) {
  return (
    <div className="error-banner" role="alert">
      <span className="error-message">{message}</span>
      {onRetry ? (
        <button type="button" className="btn btn-ghost error-retry" onClick={onRetry}>
          Retry
        </button>
      ) : null}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/** Props for {@link Loading}. */
export interface LoadingProps {
  label?: string
}

/** A simple loading indicator. */
export function Loading({ label = 'Loading…' }: LoadingProps) {
  return <div className="loading muted">{label}</div>
}

// ---------------------------------------------------------------------------
// Nullable
// ---------------------------------------------------------------------------

/** Props for {@link Nullable}. */
export interface NullableProps<T> {
  value: T | null
  render: (value: T) => ReactNode
}

/** Renders `render(value)` when non-null, otherwise the em-dash placeholder. */
export function Nullable<T>({ value, render }: NullableProps<T>): ReactNode {
  if (value === null) return <span className="muted">{PLACEHOLDER}</span>
  return render(value)
}
