// Application shell: sidebar navigation, header metrics, and content area.
import type { ReactNode } from 'react'
import { NavLink } from 'react-router-dom'
import { useSnapshotContext } from '../hooks/useMonitor'
import { DEFAULT_CONFIG, formatTimestamp } from '../types'
import type { ConnectionState } from '../types'

/** Navigation entries rendered in the sidebar, in order. */
const NAV_ITEMS: ReadonlyArray<{ to: string; label: string; end?: boolean }> = [
  { to: '/', label: 'Dashboard', end: true },
  { to: '/slots', label: 'Slots' },
  { to: '/performance', label: 'Performance' },
  { to: '/hardware', label: 'Hardware' },
  { to: '/settings', label: 'Settings' },
]

/** Props for {@link NavItem}. */
export interface NavItemProps {
  to: string
  label: string
  end?: boolean
}

/** A single sidebar link that reflects the active route. */
export function NavItem({ to, label, end = false }: NavItemProps) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) => `nav-item${isActive ? ' active' : ''}`}
    >
      {label}
    </NavLink>
  )
}

/** Props for {@link ConnectionBadge}. */
export interface ConnectionBadgeProps {
  state: ConnectionState
}

/** Coloured connection indicator; pulses only while connected. */
export function ConnectionBadge({ state }: ConnectionBadgeProps) {
  const label =
    state === 'connected' ? 'Connected' : state === 'error' ? 'Error' : 'Disconnected'
  const tone = state === 'connected' ? 'ok' : state === 'error' ? 'danger' : 'muted'
  return (
    <span className={`badge badge-${tone} connection-badge`}>
      <span className={`dot${state === 'connected' ? ' pulse' : ''}`} aria-hidden="true" />
      {label}
    </span>
  )
}

/** Props for {@link AppShell}. */
export interface AppShellProps {
  children: ReactNode
}

/** Wraps routed pages with navigation and a live status header. */
export function AppShell({ children }: AppShellProps) {
  const { snapshot } = useSnapshotContext()
  const connection: ConnectionState = snapshot?.connection ?? 'disconnected'
  const serverUrl = snapshot?.serverUrl ?? DEFAULT_CONFIG.serverUrl

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">LlamaDesk</div>
        <nav className="nav">
          {NAV_ITEMS.map((item) => (
            <NavItem key={item.to} to={item.to} label={item.label} end={item.end} />
          ))}
        </nav>
      </aside>
      <div className="main">
        <header className="topbar">
          <div className="topbar-title">LlamaDesk</div>
          <div className="topbar-meta">
            <span className="muted num">{serverUrl}</span>
            <ConnectionBadge state={connection} />
            <span className="muted num">
              Updated {formatTimestamp(snapshot?.lastUpdated ?? null)}
            </span>
          </div>
        </header>
        <main className="content">{children}</main>
      </div>
    </div>
  )
}
