import { screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Slots from './Slots'
import { resetApi } from '../test/api-stub'
import { emptySnapshot, makeSnapshot, representativeSnapshot } from '../test/fixtures'
import { renderWithSnapshot, renderWithSnapshotError } from '../test/snapshot-test-utils'

vi.mock('../lib/api', async () => {
  const { apiMock } = await import('../test/api-stub')
  return apiMock()
})

describe('Slots', () => {
  beforeEach(() => {
    resetApi()
  })

  it('should render one SlotCard for every slot in the snapshot', async () => {
    // Arrange & Act
    renderWithSnapshot(<Slots />, representativeSnapshot())

    // Assert — regression guard: all 3 slots must appear, not just the first
    const headings = await screen.findAllByRole('heading', { name: /^Slot \d/ })
    expect(headings).toHaveLength(3)
    expect(headings.map((h) => h.textContent)).toEqual(['Slot 0', 'Slot 1', 'Slot 2'])
  })

  it('should render "unlimited" for idle slots using the -1 sentinel', async () => {
    // Arrange & Act
    renderWithSnapshot(<Slots />, representativeSnapshot())

    // Assert — the fixed bug, pinned in the UI layer too
    expect(await screen.findAllByText('unlimited')).toHaveLength(2)
    expect(screen.queryByText('-1')).toBeNull()
    // The busy slot still reports a real remaining count.
    expect(screen.getByText('512')).toBeTruthy()
  })

  it('should show an empty state when no model is loaded', async () => {
    // Arrange & Act
    renderWithSnapshot(<Slots />, emptySnapshot())

    // Assert
    expect(await screen.findByText('No model loaded')).toBeTruthy()
  })

  it('should show a per-model empty state when a loaded model reports no slots', async () => {
    // Arrange & Act
    renderWithSnapshot(<Slots />, representativeSnapshot({ slots: {} }))

    // Assert
    expect(await screen.findByText('No slots')).toBeTruthy()
    expect(screen.getByText('No slots reported for qwen-3-27b.')).toBeTruthy()
  })

  it('should show an error banner when the snapshot fails to load', async () => {
    // Arrange & Act
    renderWithSnapshotError(<Slots />, 'monitor offline')

    // Assert
    expect(await screen.findByRole('alert')).toBeTruthy()
    expect(screen.getByText('monitor offline')).toBeTruthy()
  })

  it('should render the model id as the panel title', async () => {
    // Arrange & Act
    renderWithSnapshot(<Slots />, makeSnapshot({ loadedModels: ['qwen-3-27b'] }))

    // Assert
    expect(await screen.findByRole('heading', { name: 'qwen-3-27b' })).toBeTruthy()
  })
})
