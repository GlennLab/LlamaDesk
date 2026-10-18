import { describe, expect, it } from 'vitest'
import {
  DEFAULT_CONFIG,
  PLACEHOLDER,
  formatMiB,
  formatPercent,
  formatPower,
  formatRate,
  formatTemperature,
  formatTimestamp,
  formatUptime,
  isModelLoaded,
  type ModelInfo,
} from './index'

/** Builds a valid ModelInfo, overriding only the fields under test. */
function model(overrides: Partial<ModelInfo> = {}): ModelInfo {
  return {
    id: 'test-model',
    aliases: [],
    tags: [],
    status: 'unloaded',
    statusArgs: [],
    source: null,
    canRemove: false,
    contextLength: null,
    architecture: null,
    ...overrides,
  }
}

describe('PLACEHOLDER', () => {
  it('should be the em-dash placeholder used for every missing value', () => {
    expect(PLACEHOLDER).toBe('\u2014')
  })
})

describe('DEFAULT_CONFIG', () => {
  it('should match the frozen contract exactly', () => {
    expect(DEFAULT_CONFIG).toEqual({
      serverUrl: 'http://localhost:8080',
      profiles: [{ name: 'Default', serverUrl: 'http://localhost:8080' }],
      activeProfile: 'Default',
      serverIntervalMs: 2000,
      slotsIntervalMs: 1000,
      hardwareIntervalMs: 2000,
      storageIntervalMs: 5000,
      retentionHours: 24,
    })
  })
})

describe('isModelLoaded', () => {
  it('should return true when the status is the known loaded state', () => {
    expect(isModelLoaded(model({ status: 'loaded' }))).toBe(true)
  })

  it('should return false when the status is unloaded', () => {
    expect(isModelLoaded(model({ status: 'unloaded' }))).toBe(false)
  })

  it('should return false when the status is loading', () => {
    expect(isModelLoaded(model({ status: 'loading' }))).toBe(false)
  })

  it('should return false when the status is an unknown variant', () => {
    expect(isModelLoaded(model({ status: { unknown: 'sleeping' } }))).toBe(false)
  })
})

describe('formatRate', () => {
  it('should format a positive rate with the default unit', () => {
    expect(formatRate(1234.56)).toBe('1234.6 tok/s')
  })

  it('should format zero as a real rate, never the placeholder', () => {
    expect(formatRate(0)).toBe('0.0 tok/s')
    expect(formatRate(0)).not.toBe(PLACEHOLDER)
  })

  it('should preserve the sign of a negative rate', () => {
    expect(formatRate(-12.34)).toBe('-12.3 tok/s')
  })

  it('should omit the trailing space when the unit is empty', () => {
    expect(formatRate(5, '')).toBe('5.0')
  })

  it('should use a custom unit when provided', () => {
    expect(formatRate(7.25, 'req/s')).toBe('7.3 req/s')
  })

  it('should handle a very large rate without truncation', () => {
    expect(formatRate(1e9)).toBe('1000000000.0 tok/s')
  })

  it('should return the placeholder when the value is null', () => {
    expect(formatRate(null)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is undefined', () => {
    expect(formatRate(undefined as unknown as number)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is NaN', () => {
    expect(formatRate(Number.NaN)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is infinite', () => {
    expect(formatRate(Number.POSITIVE_INFINITY)).toBe(PLACEHOLDER)
  })
})

describe('formatMiB', () => {
  it('should convert MiB to GiB with one decimal', () => {
    expect(formatMiB(1024)).toBe('1.0 GiB')
    expect(formatMiB(1536)).toBe('1.5 GiB')
    expect(formatMiB(512)).toBe('0.5 GiB')
  })

  it('should format zero as a real value, never the placeholder', () => {
    expect(formatMiB(0)).toBe('0.0 GiB')
    expect(formatMiB(0)).not.toBe(PLACEHOLDER)
  })

  it('should handle a very large MiB value', () => {
    expect(formatMiB(1e9)).toBe('976562.5 GiB')
  })

  it('should return the placeholder when the value is null', () => {
    expect(formatMiB(null)).toBe(PLACEHOLDER)
    expect(formatMiB(null)).not.toBe('0.0 GiB')
  })

  it('should return the placeholder when the value is undefined', () => {
    expect(formatMiB(undefined as unknown as number)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is NaN', () => {
    expect(formatMiB(Number.NaN)).toBe(PLACEHOLDER)
  })
})

describe('formatPercent', () => {
  it('should format a percentage to one decimal with a percent sign', () => {
    expect(formatPercent(50)).toBe('50.0%')
    expect(formatPercent(100)).toBe('100.0%')
    expect(formatPercent(12.34)).toBe('12.3%')
  })

  it('should format zero as a real percentage, never the placeholder', () => {
    expect(formatPercent(0)).toBe('0.0%')
    expect(formatPercent(0)).not.toBe(PLACEHOLDER)
  })

  it('should preserve the sign of a negative percentage', () => {
    expect(formatPercent(-5)).toBe('-5.0%')
  })

  it('should return the placeholder when the value is null', () => {
    expect(formatPercent(null)).toBe(PLACEHOLDER)
    expect(formatPercent(null)).not.toBe('0.0%')
  })

  it('should return the placeholder when the value is undefined', () => {
    expect(formatPercent(undefined as unknown as number)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is NaN', () => {
    expect(formatPercent(Number.NaN)).toBe(PLACEHOLDER)
  })
})

describe('formatTemperature', () => {
  it('should format a temperature to one decimal Celsius', () => {
    // 21.25 is exactly representable, so rounding to one decimal is deterministic.
    expect(formatTemperature(21.25)).toBe('21.3\u00b0C')
    expect(formatTemperature(72.5)).toBe('72.5\u00b0C')
  })

  it('should format zero as a real temperature, never the placeholder', () => {
    expect(formatTemperature(0)).toBe('0.0\u00b0C')
    expect(formatTemperature(0)).not.toBe(PLACEHOLDER)
  })

  it('should preserve the sign of a negative temperature', () => {
    expect(formatTemperature(-10)).toBe('-10.0\u00b0C')
  })

  it('should return the placeholder when the value is null', () => {
    expect(formatTemperature(null)).toBe(PLACEHOLDER)
    expect(formatTemperature(null)).not.toBe('0.0\u00b0C')
  })

  it('should return the placeholder when the value is undefined', () => {
    expect(formatTemperature(undefined as unknown as number)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is NaN', () => {
    expect(formatTemperature(Number.NaN)).toBe(PLACEHOLDER)
  })
})

describe('formatPower', () => {
  it('should format only the draw when no limit is supplied', () => {
    expect(formatPower(100)).toBe('100 W')
  })

  it('should round the draw to whole watts', () => {
    expect(formatPower(99.6)).toBe('100 W')
  })

  it('should format draw and limit when both are supplied', () => {
    expect(formatPower(100, 250)).toBe('100 W / 250 W')
  })

  it('should format zero draw as a real value, never the placeholder', () => {
    expect(formatPower(0)).toBe('0 W')
    expect(formatPower(0)).not.toBe(PLACEHOLDER)
  })

  it('should format zero draw alongside a non-null limit', () => {
    expect(formatPower(0, 400)).toBe('0 W / 400 W')
  })

  it('should omit the limit when it is null', () => {
    expect(formatPower(100, null)).toBe('100 W')
  })

  it('should omit the limit when it is not finite', () => {
    expect(formatPower(100, Number.NaN)).toBe('100 W')
    expect(formatPower(100, Number.POSITIVE_INFINITY)).toBe('100 W')
  })

  it('should return the placeholder when the draw is null', () => {
    expect(formatPower(null)).toBe(PLACEHOLDER)
    expect(formatPower(null)).not.toBe('0 W')
  })

  it('should return the placeholder when the draw is undefined', () => {
    expect(formatPower(undefined as unknown as number)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the draw is NaN', () => {
    expect(formatPower(Number.NaN, 250)).toBe(PLACEHOLDER)
  })
})

describe('formatUptime', () => {
  it('should format sub-minute durations as seconds', () => {
    expect(formatUptime(0)).toBe('0s')
    expect(formatUptime(59)).toBe('59s')
  })

  it('should format minute-scale durations', () => {
    expect(formatUptime(60)).toBe('1m 0s')
    expect(formatUptime(61)).toBe('1m 1s')
  })

  it('should format hour-scale durations', () => {
    expect(formatUptime(3600)).toBe('1h 0m')
    expect(formatUptime(3661)).toBe('1h 1m')
  })

  it('should format day-scale durations', () => {
    expect(formatUptime(86400)).toBe('1d 0h 0m')
    expect(formatUptime(90061)).toBe('1d 1h 1m')
  })

  it('should floor fractional seconds', () => {
    expect(formatUptime(90.9)).toBe('1m 30s')
  })

  it('should return the placeholder when the value is null', () => {
    expect(formatUptime(null)).toBe(PLACEHOLDER)
    expect(formatUptime(null)).not.toBe('0s')
  })

  it('should return the placeholder when the value is undefined', () => {
    expect(formatUptime(undefined as unknown as number)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder for a negative duration', () => {
    expect(formatUptime(-1)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the value is NaN', () => {
    expect(formatUptime(Number.NaN)).toBe(PLACEHOLDER)
  })
})

describe('formatTimestamp', () => {
  it('should format a valid ISO timestamp as local time', () => {
    const iso = '2026-10-15T23:00:00.000Z'
    expect(formatTimestamp(iso)).toBe(new Date(iso).toLocaleTimeString())
  })

  it('should return the placeholder when the timestamp is null', () => {
    expect(formatTimestamp(null)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the timestamp is undefined', () => {
    expect(formatTimestamp(undefined as unknown as string)).toBe(PLACEHOLDER)
  })

  it('should return the placeholder when the timestamp cannot be parsed', () => {
    expect(formatTimestamp('not-a-date')).toBe(PLACEHOLDER)
  })
})
