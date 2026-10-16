import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { isModelLoaded } from '../types'
import { GpuCard, MetricGrid, ModelRow, SlotCard } from './cards'
import { makeGpu, makeModel, makeNextToken, makeSlot } from '../test/fixtures'

describe('SlotCard', () => {
  it('should render "unlimited" when tokens remaining is the negative sentinel', () => {
    // Arrange
    render(
      <SlotCard
        index={0}
        slot={makeSlot({
          id: 0,
          isProcessing: false,
          nextToken: [makeNextToken({ nRemain: -1, nDecoded: 0 })],
        })}
      />,
    )

    // Assert — idle slots send -1 to mean "no limit", never a negative count
    const row = screen.getByText('Tokens remaining').closest('.metric-row') as HTMLElement
    expect(within(row).getByText('unlimited')).toBeTruthy()
    expect(within(row).queryByText('-1')).toBeNull()
  })

  it('should render the placeholder for a null tokens-remaining value, never zero', () => {
    // Arrange
    render(
      <SlotCard index={0} slot={makeSlot({ nextToken: [makeNextToken({ nRemain: null })] })} />,
    )

    // Assert
    const row = screen.getByText('Tokens remaining').closest('.metric-row') as HTMLElement
    expect(within(row).getByText('—')).toBeTruthy()
    expect(within(row).queryByText('0')).toBeNull()
  })

  it('should render zero tokens remaining as the number 0', () => {
    // Arrange
    render(
      <SlotCard index={0} slot={makeSlot({ nextToken: [makeNextToken({ nRemain: 0 })] })} />,
    )

    // Assert — a real zero is shown, not the placeholder and not "unlimited"
    const row = screen.getByText('Tokens remaining').closest('.metric-row') as HTMLElement
    expect(within(row).getByText('0')).toBeTruthy()
    expect(within(row).queryByText('unlimited')).toBeNull()
    expect(within(row).queryByText('—')).toBeNull()
  })

  it('should render a positive tokens-remaining count as a number', () => {
    // Arrange
    render(
      <SlotCard index={0} slot={makeSlot({ nextToken: [makeNextToken({ nRemain: 42 })] })} />,
    )

    // Assert
    const row = screen.getByText('Tokens remaining').closest('.metric-row') as HTMLElement
    expect(within(row).getByText('42')).toBeTruthy()
  })

  it('should show a processing badge and prompt progress for an active slot', () => {
    // Arrange
    render(
      <SlotCard
        index={0}
        slot={makeSlot({
          id: 3,
          isProcessing: true,
          nPromptTokens: 100,
          nPromptTokensProcessed: 50,
          nextToken: [makeNextToken({ nRemain: 10 })],
        })}
      />,
    )

    // Assert
    expect(screen.getByText('processing')).toBeTruthy()
    expect(screen.getByText('50.0%')).toBeTruthy()
  })

  it('should render an idle slot as idle with a placeholder prompt progress', () => {
    // Arrange
    render(<SlotCard index={2} slot={makeSlot({ id: 2, isProcessing: false, nextToken: [] })} />)

    // Assert
    expect(screen.getByText('idle')).toBeTruthy()
    expect(screen.getByText('Prompt progress')).toBeTruthy()
    const row = screen.getByText('Prompt progress').closest('.metric-row') as HTMLElement
    expect(within(row).getByText('—')).toBeTruthy()
  })
})

describe('GpuCard', () => {
  it('should render placeholders, not NaN, when every measurement is missing', () => {
    // Arrange & Act
    const { container } = render(<GpuCard gpu={makeGpu({ index: 0, name: 'RTX 4090' })} />)

    // Assert
    expect(screen.getByRole('heading', { name: 'GPU 0' })).toBeTruthy()
    expect(container.textContent).not.toContain('NaN')
    expect(screen.getAllByText('—').length).toBeGreaterThanOrEqual(4)
    expect(screen.queryByText('0.0%')).toBeNull()
  })

  it('should format utilization, VRAM, temperature and power when present', () => {
    // Arrange & Act
    render(
      <GpuCard
        gpu={makeGpu({
          index: 0,
          name: 'RTX 4090',
          utilizationPercent: 50,
          memoryUsedMib: 1024,
          memoryTotalMib: 2048,
          temperatureC: 65,
          powerDrawW: 120,
          powerLimitW: 300,
        })}
      />,
    )

    // Assert
    expect(screen.getByText('65.0°C')).toBeTruthy()
    expect(screen.getByText('120 / 300 W')).toBeTruthy()
    expect(screen.getByText(/1\.0 GiB \/ 2\.0 GiB/)).toBeTruthy()
    expect(screen.getAllByText('50.0%').length).toBeGreaterThanOrEqual(2)
    expect(screen.queryByText('NaN')).toBeNull()
  })

  it('should flag a hot GPU with the warn temperature tone', () => {
    // Arrange & Act
    render(<GpuCard gpu={makeGpu({ index: 0, temperatureC: 85 })} />)

    // Assert
    expect(screen.getByText('85.0°C').className).toContain('badge-warn')
  })

  it('should render power without a limit when only the draw is known', () => {
    // Arrange & Act
    render(<GpuCard gpu={makeGpu({ index: 0, powerDrawW: 120 })} />)

    // Assert
    expect(screen.getByText('120 W')).toBeTruthy()
  })
})

describe('ModelRow', () => {
  it('should render a loaded and an unloaded model differently', () => {
    // Arrange
    const loaded = makeModel({
      id: 'loaded-model',
      status: 'loaded',
      canRemove: true,
      contextLength: 4096,
      source: '/m/a.gguf',
    })
    const unloaded = makeModel({ id: 'idle-model', status: 'unloaded' })
    expect(isModelLoaded(loaded)).toBe(true)
    expect(isModelLoaded(unloaded)).toBe(false)

    // Act
    const { unmount } = render(<ModelRow model={loaded} />)

    // Assert
    expect(screen.getByText('loaded').className).toContain('badge-ok')

    // Act
    unmount()
    render(<ModelRow model={unloaded} />)

    // Assert
    expect(screen.getByText('unloaded').className).toContain('badge-default')
  })

  it('should render an unknown status object as a danger badge with its raw value', () => {
    // Arrange & Act
    render(<ModelRow model={makeModel({ status: { unknown: 'draining' } })} />)

    // Assert
    expect(screen.getByText('unknown: draining')).toBeTruthy()
    expect(screen.getByText('unknown: draining').className).toContain('badge-danger')
  })

  it('should render placeholders for a missing context length and source', () => {
    // Arrange & Act
    render(<ModelRow model={makeModel({ contextLength: null, source: null, canRemove: false })} />)

    // Assert
    const row = screen.getByText('model').closest('.metric-row') as HTMLElement
    expect(within(row).getAllByText('—').length).toBe(2)
    expect(within(row).getByText('locked')).toBeTruthy()
  })
})

describe('MetricGrid', () => {
  it('should render its children inside a responsive grid', () => {
    // Arrange & Act
    const { container } = render(
      <MetricGrid>
        <span>one</span>
        <span>two</span>
      </MetricGrid>,
    )

    // Assert
    expect(screen.getByText('one')).toBeTruthy()
    expect(screen.getByText('two')).toBeTruthy()
    expect((container.firstChild as HTMLElement).style.gridTemplateColumns).toContain('repeat')
  })
})
