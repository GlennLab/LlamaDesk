import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import {
  Badge,
  EmptyState,
  ErrorBanner,
  Loading,
  MetricRow,
  Nullable,
  Panel,
  StatCard,
} from './ui'

describe('Nullable', () => {
  it('should render the placeholder when the value is null', () => {
    // Arrange & Act
    render(<Nullable value={null} render={(v) => <span>{v}</span>} />)

    // Assert
    expect(screen.getByText('—')).toBeTruthy()
  })

  it('should render the placeholder when the value is undefined (absent field)', () => {
    // Arrange — an absent JSON field arrives at runtime as `undefined`, which
    // the public `T | null` prop type cannot express; the cast models that boundary.
    const absent = undefined as unknown as string | null
    const renderSpy = vi.fn((v: string) => <span>{v}</span>)

    // Act
    render(<Nullable value={absent} render={renderSpy} />)

    // Assert — absence renders the placeholder and never reaches render
    expect(screen.getByText('—')).toBeTruthy()
    expect(renderSpy).not.toHaveBeenCalled()
  })

  it('should render zero as a real value, not the placeholder', () => {
    // Arrange & Act
    render(<Nullable value={0} render={(v) => <span>{v}</span>} />)

    // Assert — the null-vs-zero contract: 0 is data, not absence
    expect(screen.getByText('0')).toBeTruthy()
    expect(screen.queryByText('—')).toBeNull()
  })

  it('should render false as a real value, not the placeholder', () => {
    // Arrange & Act
    render(<Nullable value={false} render={(v) => <span>{String(v)}</span>} />)

    // Assert — false is data, only null is absence
    expect(screen.getByText('false')).toBeTruthy()
    expect(screen.queryByText('—')).toBeNull()
  })

  it('should render what the render prop returns for a non-null value', () => {
    // Arrange & Act
    render(<Nullable value="ready" render={(v) => <span>state:{v}</span>} />)

    // Assert
    expect(screen.getByText('state:ready')).toBeTruthy()
  })
})

describe('StatCard', () => {
  it('should render the label, value, unit and hint', () => {
    // Arrange & Act
    render(<StatCard label="Speed" value="12.3" unit="tok/s" hint="primary" tone="ok" />)

    // Assert
    expect(screen.getByText('Speed')).toBeTruthy()
    expect(screen.getByText('12.3')).toBeTruthy()
    expect(screen.getByText('tok/s')).toBeTruthy()
    expect(screen.getByText('primary')).toBeTruthy()
  })

  it('should apply the requested tone class and default to the default tone', () => {
    // Arrange
    const { container: defaultCard } = render(<StatCard label="A" value="1" />)
    const { container: dangerCard } = render(<StatCard label="B" value="2" tone="danger" />)

    // Assert
    expect((defaultCard.firstChild as HTMLElement).className).toContain('stat-default')
    expect((dangerCard.firstChild as HTMLElement).className).toContain('stat-danger')
  })
})

describe('MetricRow', () => {
  it('should render the label and the value together', () => {
    // Arrange & Act
    render(<MetricRow label="Task id" value="42" />)

    // Assert
    const row = screen.getByText('Task id').closest('.metric-row') as HTMLElement
    expect(within(row).getByText('42')).toBeTruthy()
  })
})

describe('Badge', () => {
  it('should render its children with the requested tone class', () => {
    // Arrange & Act
    const { container } = render(<Badge tone="warn">waiting</Badge>)

    // Assert
    expect(screen.getByText('waiting')).toBeTruthy()
    expect((container.firstChild as HTMLElement).className).toContain('badge-warn')
  })
})

describe('EmptyState', () => {
  it('should render the title and the message', () => {
    // Arrange & Act
    render(<EmptyState title="No data" message="Nothing here yet." />)

    // Assert
    expect(screen.getByText('No data')).toBeTruthy()
    expect(screen.getByText('Nothing here yet.')).toBeTruthy()
  })
})

describe('ErrorBanner', () => {
  it('should expose the message as an alert', () => {
    // Arrange & Act
    render(<ErrorBanner message="boom" />)

    // Assert
    expect(screen.getByRole('alert')).toBeTruthy()
    expect(screen.getByText('boom')).toBeTruthy()
  })

  it('should render a Retry button that calls onRetry when provided', () => {
    // Arrange
    const onRetry = vi.fn()
    render(<ErrorBanner message="boom" onRetry={onRetry} />)

    // Act
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }))

    // Assert
    expect(onRetry).toHaveBeenCalledTimes(1)
  })

  it('should not render a Retry button when no onRetry is provided', () => {
    // Arrange & Act
    render(<ErrorBanner message="boom" />)

    // Assert
    expect(screen.queryByRole('button')).toBeNull()
  })
})

describe('Loading', () => {
  it('should render the default label', () => {
    // Arrange & Act
    render(<Loading />)

    // Assert
    expect(screen.getByText('Loading…')).toBeTruthy()
  })

  it('should render a custom label', () => {
    // Arrange & Act
    render(<Loading label="Loading slots…" />)

    // Assert
    expect(screen.getByText('Loading slots…')).toBeTruthy()
  })
})

describe('Panel', () => {
  it('should render the title, subtitle, actions and children', () => {
    // Arrange & Act
    render(
      <Panel title="Server" subtitle="live" actions={<button type="button">go</button>}>
        <p>body</p>
      </Panel>,
    )

    // Assert
    expect(screen.getByRole('heading', { name: 'Server' })).toBeTruthy()
    expect(screen.getByText('live')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'go' })).toBeTruthy()
    expect(screen.getByText('body')).toBeTruthy()
  })

  it('should omit subtitle and actions when they are not provided', () => {
    // Arrange & Act
    render(
      <Panel title="Server">
        <p>body</p>
      </Panel>,
    )

    // Assert
    expect(screen.getByRole('heading', { name: 'Server' })).toBeTruthy()
    expect(screen.queryByRole('button')).toBeNull()
  })
})
