// Render helper for components/pages that read the live monitor snapshot.
//
// It wraps the UI in the *real* `SnapshotProvider` (so `useSnapshot` and
// `useSnapshotContext` run for real) and feeds it a supplied snapshot through
// the mocked Tauri bridge. The context is never stubbed, so provider bugs stay
// visible. A `MemoryRouter` is included because routed components (e.g. the
// app shell's `NavLink`s) need a router.
import type { ReactElement } from 'react'
import { render } from '@testing-library/react'
import type { RenderResult } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { SnapshotProvider } from '../hooks/useMonitor'
import type { MonitorSnapshot } from '../types'
import { api } from './api-stub'

/**
 * Renders `ui` with `snapshot` delivered by the real `SnapshotProvider`.
 *
 * Consumers should await a `findBy*`/`findAllBy*` query: the provider resolves
 * the snapshot asynchronously, exactly as it does in production.
 */
export function renderWithSnapshot(
  ui: ReactElement,
  snapshot: MonitorSnapshot,
  options: { route?: string } = {},
): RenderResult {
  api.getSnapshot.mockResolvedValue(snapshot)
  api.subscribeSnapshot.mockResolvedValue(() => {})
  return render(
    <MemoryRouter initialEntries={[options.route ?? '/']}>
      <SnapshotProvider>{ui}</SnapshotProvider>
    </MemoryRouter>,
  )
}

/**
 * Renders `ui` when the initial snapshot fetch fails, so the page's
 * graceful-degradation (error) branch is exercised with no snapshot present.
 */
export function renderWithSnapshotError(ui: ReactElement, message: string): RenderResult {
  api.getSnapshot.mockRejectedValue(message)
  api.subscribeSnapshot.mockResolvedValue(() => {})
  return render(
    <MemoryRouter>
      <SnapshotProvider>{ui}</SnapshotProvider>
    </MemoryRouter>,
  )
}
