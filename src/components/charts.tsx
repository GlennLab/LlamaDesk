// Thin, typed wrapper over echarts-for-react with the app's dark theme.
// Datasets are `[timestampMs, value]` pairs on a `time` axis; `null` values
// are passed through (connectNulls: false) so gaps stay visible — never zeroed.
import ReactECharts from 'echarts-for-react'
import { PLACEHOLDER } from '../types'

/** A single chart series. `type: 'area'` fills below the line. */
export interface ChartSeries {
  name: string
  data: [number, number | null][]
  color: string
  type?: 'line' | 'area'
}

/** Props for {@link TimelineChart}. */
export interface TimelineChartProps {
  title?: string
  series: ChartSeries[]
  yAxisName?: string
  valueFormatter?: (v: number) => string
  height?: number
}

/** The subset of an axis-tooltip callback item this formatter relies on. */
interface AxisTooltipParam {
  seriesName?: string
  value?: unknown
  color?: string
}

const AXIS_LABEL = '#8b949e'
const GRID_LINE = '#232a33'
const PANEL_ALT = '#1b212b'
const TEXT = '#e6edf3'

/** Renders the axis tooltip: a timestamp header plus one line per series. */
function formatTooltip(params: AxisTooltipParam[], fmt: (v: number) => string): string {
  const first = params[0]?.value
  const ts = Array.isArray(first) ? (first as unknown[])[0] : null
  const header = typeof ts === 'number' ? new Date(ts).toLocaleTimeString() : ''
  const lines = params.map((p) => {
    const pair = p.value
    const v = Array.isArray(pair) ? (pair as unknown[])[1] : null
    const label = typeof v === 'number' && Number.isFinite(v) ? fmt(v) : PLACEHOLDER
    return `${p.seriesName ?? ''}: ${label}`
  })
  return [header, ...lines].join('<br/>')
}

/** A multi-series time chart with data zoom, legends and gap-preserving nulls. */
export function TimelineChart({
  title,
  series,
  yAxisName,
  valueFormatter,
  height = 260,
}: TimelineChartProps) {
  const fmt = valueFormatter ?? ((v: number): string => String(v))

  const option = {
    backgroundColor: 'transparent',
    title: title
      ? { text: title, left: 0, top: 0, textStyle: { color: TEXT, fontSize: 13, fontWeight: 600 } }
      : undefined,
    legend:
      series.length > 1
        ? { right: 0, top: 0, textStyle: { color: AXIS_LABEL }, icon: 'roundRect' }
        : undefined,
    grid: { left: 8, right: 16, top: title ? 40 : 16, bottom: 44, containLabel: true },
    tooltip: {
      trigger: 'axis',
      backgroundColor: PANEL_ALT,
      borderColor: GRID_LINE,
      textStyle: { color: TEXT },
      formatter: (params: AxisTooltipParam[]): string => formatTooltip(params, fmt),
    },
    xAxis: {
      type: 'time',
      axisLabel: { color: AXIS_LABEL },
      axisLine: { lineStyle: { color: GRID_LINE } },
      splitLine: { show: false },
    },
    yAxis: {
      type: 'value',
      name: yAxisName,
      nameTextStyle: { color: AXIS_LABEL },
      axisLabel: { color: AXIS_LABEL },
      splitLine: { lineStyle: { color: GRID_LINE } },
    },
    dataZoom: [
      { type: 'inside' },
      {
        type: 'slider',
        height: 18,
        bottom: 8,
        borderColor: GRID_LINE,
        backgroundColor: 'transparent',
        fillerColor: 'rgba(76, 154, 255, 0.15)',
        handleStyle: { color: '#4c9aff' },
        textStyle: { color: AXIS_LABEL },
      },
    ],
    series: series.map((s) => ({
      name: s.name,
      type: 'line',
      showSymbol: false,
      connectNulls: false,
      smooth: false,
      lineStyle: { color: s.color, width: 2 },
      itemStyle: { color: s.color },
      areaStyle: s.type === 'area' ? { color: s.color, opacity: 0.15 } : undefined,
      data: s.data,
    })),
  }

  return <ReactECharts option={option} style={{ height }} notMerge lazyUpdate />
}
