import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Performance from './Performance'
import { api, resetApi } from '../test/api-stub'
import { makeGpuSample, makeServerSample, representativeSnapshot } from '../test/fixtures'
import { renderWithSnapshot } from '../test/snapshot-test-utils'

vi.mock('../lib/api', async () => {
  const { apiMock } = await import('../test/api-stub')
  return apiMock()
})

// ECharts is too heavy for jsdom; capture the options the page forwards.
const echartsMock = vi.hoisted(() => vi.fn((_props: { option: unknown }) => null))
vi.mock('echarts-for-react', () => ({ default: echartsMock }))

function chartTitles(): Array<string | undefined> {
  return echartsMock.mock.calls.map(
    (call) => (call[0].option as { title?: { text?: string } }).title?.text,
  )
}

const SERVER_SAMPLES = [
  makeServerSample({ timestampMs: 1000, promptTokensPerSec: 10, generationTokensPerSec: 5, activeRequests: 1 }),
  makeServerSample({ timestampMs: 2000, promptTokensPerSec: 20, generationTokensPerSec: 8, activeRequests: 2 }),
]
const GPU_SAMPLES = [
  makeGpuSample({ timestampMs: 1000, gpuIndex: 0, utilizationPercent: 50, memoryUsedMib: 1024 }),
  makeGpuSample({ timestampMs: 2000, gpuIndex: 0, utilizationPercent: 60, memoryUsedMib: 1200 }),
]

describe('Performance', () => {
  beforeEach(() => {
    resetApi()
    echartsMock.mockClear()
  })

  it('should render the history panel, range selector and one chart per metric', async () => {
    // Arrange
    api.getServerHistory.mockResolvedValue(SERVER_SAMPLES)
    api.getGpuHistory.mockResolvedValue(GPU_SAMPLES)

    // Act
    renderWithSnapshot(<Performance />, representativeSnapshot())

    // Assert
    expect(await screen.findByRole('heading', { name: 'Performance history' })).toBeTruthy()
    expect(screen.getByRole('combobox', { name: 'Time range' })).toBeTruthy()
    await waitFor(() => expect(echartsMock).toHaveBeenCalledTimes(8))
    expect(chartTitles()).toEqual(
      expect.arrayContaining([
        'Prompt processing',
        'Generation speed',
        'Active requests',
        'Deferred requests',
        'GPU utilization',
        'GPU VRAM used',
        'GPU fan speed',
        'GPU temperature',
      ]),
    )
  })

  it('should default the range selector to the last hour', async () => {
    // Arrange & Act
    renderWithSnapshot(<Performance />, representativeSnapshot())
    const select = (await screen.findByRole('combobox', { name: 'Time range' })) as HTMLSelectElement

    // Assert
    expect(select.value).toBe(String(60 * 60_000))
  })

  it('should reload history when a different range is selected', async () => {
    // Arrange
    renderWithSnapshot(<Performance />, representativeSnapshot())
    await waitFor(() => expect(api.getServerHistory).toHaveBeenCalledTimes(1))

    // Act
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Time range' }),
      String(15 * 60_000),
    )

    // Assert
    await waitFor(() => expect(api.getServerHistory).toHaveBeenCalledTimes(2))
  })

  it('should re-fetch both histories when Refresh is clicked', async () => {
    // Arrange
    renderWithSnapshot(<Performance />, representativeSnapshot())
    const refresh = screen.getByRole('button', { name: 'Refresh' }) as HTMLButtonElement
    await waitFor(() => expect(refresh.disabled).toBe(false))
    await waitFor(() => expect(api.getServerHistory).toHaveBeenCalledTimes(1))

    // Act
    await userEvent.click(refresh)

    // Assert
    await waitFor(() => expect(api.getServerHistory).toHaveBeenCalledTimes(2))
    expect(api.getGpuHistory).toHaveBeenCalledTimes(2)
  })

  it('should show an empty state when no history is available', async () => {
    // Arrange & Act
    renderWithSnapshot(<Performance />, representativeSnapshot())

    // Assert
    expect(await screen.findByText('No history yet')).toBeTruthy()
    expect(screen.queryByText('NaN')).toBeNull()
  })

  it('should surface a history error as an alert', async () => {
    // Arrange
    api.getServerHistory.mockRejectedValue('database is locked')

    // Act
    renderWithSnapshot(<Performance />, representativeSnapshot())

    // Assert
    expect(await screen.findByRole('alert')).toBeTruthy()
    expect(screen.getByText('database is locked')).toBeTruthy()
  })
})
