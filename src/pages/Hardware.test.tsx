import { screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Hardware from './Hardware'
import { resetApi } from '../test/api-stub'
import { makeGpu, representativeSnapshot } from '../test/fixtures'
import { renderWithSnapshot, renderWithSnapshotError } from '../test/snapshot-test-utils'

vi.mock('../lib/api', async () => {
  const { apiMock } = await import('../test/api-stub')
  return apiMock()
})

describe('Hardware', () => {
  beforeEach(() => {
    resetApi()
  })

  it('should render the totals and a card per GPU', async () => {
    // Arrange & Act
    renderWithSnapshot(<Hardware />, representativeSnapshot())

    // Assert
    expect(await screen.findByRole('heading', { name: 'GPU 0' })).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'GPU 1' })).toBeTruthy()
    const gpuCount = screen.getByText('GPUs').closest('.stat-card') as HTMLElement
    expect(within(gpuCount).getByText('2')).toBeTruthy()
  })

  it('should render em-dashes and no NaN for a GPU with missing values', async () => {
    // Arrange & Act
    const { container } = renderWithSnapshot(
      <Hardware />,
      representativeSnapshot({ gpus: [makeGpu({ index: 0, name: 'RTX 4090' })] }),
    )

    // Assert
    expect(await screen.findByRole('heading', { name: 'GPU 0' })).toBeTruthy()
    expect(container.textContent).not.toContain('NaN')
    expect(screen.getAllByText('—').length).toBeGreaterThanOrEqual(3)
    // Missing VRAM totals must not be coerced to 0.0 GiB.
    expect(screen.queryByText('0.0 GiB')).toBeNull()
  })

  it('should show an error banner when the snapshot fails to load', async () => {
    // Arrange & Act
    renderWithSnapshotError(<Hardware />, 'monitor offline')

    // Assert
    expect(await screen.findByRole('alert')).toBeTruthy()
    expect(screen.getByText('monitor offline')).toBeTruthy()
  })

  it('should show the unavailable empty state when no GPUs are available', async () => {
    // Arrange & Act
    renderWithSnapshot(
      <Hardware />,
      representativeSnapshot({ gpus: [], gpuAvailable: false }),
    )

    // Assert
    expect(await screen.findByText('GPU monitoring unavailable')).toBeTruthy()
  })
})
