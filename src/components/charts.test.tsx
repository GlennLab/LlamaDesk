// ECharts is heavy under jsdom, so `echarts-for-react` is replaced with a spy.
// We assert only on the options TimelineChart forwards — never ECharts internals.
import { render } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { TimelineChart } from './charts'
import type { ChartSeries } from './charts'

// Hoisted so the vi.mock factory can reference it. The underscore param keeps
// `noUnusedParameters` happy while preserving the props type at the call site.
const echartsMock = vi.hoisted(() => vi.fn((_props: { option: unknown }) => null))

vi.mock('echarts-for-react', () => ({ default: echartsMock }))

interface CapturedSeries {
  name: string
  connectNulls: boolean
  data: [number, number | null][]
}

interface CapturedOption {
  title?: { text?: string }
  legend?: unknown
  series: CapturedSeries[]
  tooltip: { formatter: (params: Array<{ seriesName?: string; value?: unknown }>) => string }
}

/** The option object handed to the most recent chart render. */
function lastOption(): CapturedOption {
  const calls = echartsMock.mock.calls
  return calls[calls.length - 1][0].option as CapturedOption
}

function series(name: string, data: [number, number | null][]): ChartSeries {
  return { name, color: '#111', data }
}

beforeEach(() => {
  echartsMock.mockClear()
})

describe('TimelineChart', () => {
  it('should forward one series per input and preserve null gaps', () => {
    // Arrange & Act
    render(
      <TimelineChart
        title="Rates"
        series={[
          series('a', [
            [1000, 10],
            [2000, null],
          ]),
          series('b', [
            [1000, null],
            [2000, 20],
          ]),
        ]}
      />,
    )

    // Assert — gaps must stay gaps: connectNulls is always false
    const option = lastOption()
    expect(option.series).toHaveLength(2)
    expect(option.series.every((s) => s.connectNulls === false)).toBe(true)
    expect(option.series[0].data).toEqual([
      [1000, 10],
      [2000, null],
    ])
  })

  it('should show a legend only when there is more than one series', () => {
    // Arrange
    const one = [series('a', [[1, 1]])]
    const two = [series('a', [[1, 1]]), series('b', [[1, 2]])]

    // Act
    const { unmount } = render(<TimelineChart series={one} />)
    const singleLegend = lastOption().legend
    unmount()
    render(<TimelineChart series={two} />)

    // Assert
    expect(singleLegend).toBeUndefined()
    expect(lastOption().legend).toBeTruthy()
  })

  it('should include the title when provided', () => {
    // Arrange & Act
    render(<TimelineChart title="Prompt processing" series={[]} />)

    // Assert
    expect(lastOption().title?.text).toBe('Prompt processing')
  })

  it('should tolerate an empty series list without throwing', () => {
    // Arrange & Act
    render(<TimelineChart series={[]} />)

    // Assert
    expect(lastOption().series).toEqual([])
    expect(echartsMock).toHaveBeenCalledTimes(1)
  })

  it('should tolerate a single-point series', () => {
    // Arrange & Act
    render(<TimelineChart series={[series('a', [[1000, 5]])]} />)

    // Assert
    expect(lastOption().series[0].data).toEqual([[1000, 5]])
  })

  it('should keep nulls and not throw when every value is null', () => {
    // Arrange & Act
    render(
      <TimelineChart
        series={[
          series('a', [
            [1000, null],
            [2000, null],
          ]),
        ]}
      />,
    )

    // Assert
    const s = lastOption().series[0]
    expect(s.data).toEqual([
      [1000, null],
      [2000, null],
    ])
    expect(s.connectNulls).toBe(false)
  })

  it('should format tooltip values and show the placeholder for nulls', () => {
    // Arrange & Act
    render(
      <TimelineChart
        series={[series('a', [[1000, 12.5]])]}
        valueFormatter={(v) => `${v.toFixed(1)} tok/s`}
      />,
    )
    const fmt = lastOption().tooltip.formatter

    // Assert
    expect(fmt([{ seriesName: 'a', value: [1000, 12.5] }])).toContain('a: 12.5 tok/s')
    expect(fmt([{ seriesName: 'a', value: [1000, null] }])).toContain('a: —')
  })
})
