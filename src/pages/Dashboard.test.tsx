import { screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Dashboard from './Dashboard'
import { resetApi } from '../test/api-stub'
import { emptySnapshot, representativeSnapshot } from '../test/fixtures'
import { renderWithSnapshot, renderWithSnapshotError } from '../test/snapshot-test-utils'

vi.mock('../lib/api', async () => {
  const { apiMock } = await import('../test/api-stub')
  return apiMock()
})

describe('Dashboard', () => {
  beforeEach(() => {
    resetApi()
  })

  it('should render connection, server and model details from a snapshot', async () => {
    // Arrange & Act
    renderWithSnapshot(<Dashboard />, representativeSnapshot())

    // Assert
    expect(await screen.findByText('router')).toBeTruthy()
    expect(screen.getByText('b2794-62df1815')).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'Connection' })).toBeTruthy()
    expect(screen.getByText('Connected')).toBeTruthy()

    const modelsRow = screen.getByText('Models').closest('.metric-row') as HTMLElement
    expect(within(modelsRow).getByText('2')).toBeTruthy()

    // The loaded model is surfaced as an active-model badge (and metric hints).
    expect(screen.getAllByText('qwen-3-27b').length).toBeGreaterThanOrEqual(1)

    // 3 slots, 1 processing -> 2 free slots.
    const freeSlots = screen.getByText('Free slots').closest('.stat-card') as HTMLElement
    expect(within(freeSlots).getByText('2')).toBeTruthy()

    // GPU memory summary lists each GPU.
    expect(screen.getByText('GPU 0')).toBeTruthy()
    expect(screen.getByText('GPU 1')).toBeTruthy()
  })

  it('should surface per-slice errors as alerts', async () => {
    // Arrange & Act
    renderWithSnapshot(
      <Dashboard />,
      representativeSnapshot({ errors: [{ slice: 'slots', message: 'timed out' }] }),
    )

    // Assert
    expect(await screen.findByRole('alert')).toBeTruthy()
    expect(screen.getByText('slots: timed out')).toBeTruthy()
  })

  it('should show empty states, no NaN and no zero-for-missing when disconnected', async () => {
    // Arrange & Act
    const { container } = renderWithSnapshot(<Dashboard />, emptySnapshot())

    // Assert
    expect(await screen.findByText('No model loaded')).toBeTruthy()
    expect(screen.getByText('NVIDIA monitoring is unavailable.')).toBeTruthy()
    expect(container.textContent).not.toContain('NaN')

    // A genuinely-known zero (model count) stays 0 ...
    const modelsRow = screen.getByText('Models').closest('.metric-row') as HTMLElement
    expect(within(modelsRow).getByText('0')).toBeTruthy()

    // ... while a missing metric is the placeholder, never 0.
    const activeRequests = screen.getByText('Active requests').closest('.stat-card') as HTMLElement
    expect(within(activeRequests).getByText('—')).toBeTruthy()
    expect(within(activeRequests).queryByText('0')).toBeNull()
  })

  it('should show an error banner when the snapshot fails to load', async () => {
    // Arrange & Act
    renderWithSnapshotError(<Dashboard />, 'monitor offline')

    // Assert
    expect(await screen.findByRole('alert')).toBeTruthy()
    expect(screen.getByText('monitor offline')).toBeTruthy()
  })

  it('should render the ghost row for an empty GPU list when GPUs are available', async () => {
    // Arrange & Act
    renderWithSnapshot(
      <Dashboard />,
      representativeSnapshot({ gpus: [], gpuAvailable: true }),
    )

    // Assert
    expect(await screen.findByText('No GPUs were detected.')).toBeTruthy()
  })
})
