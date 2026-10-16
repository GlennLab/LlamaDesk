import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AppShell, ConnectionBadge } from './layout'
import { resetApi } from '../test/api-stub'
import { emptySnapshot, representativeSnapshot } from '../test/fixtures'
import { renderWithSnapshot } from '../test/snapshot-test-utils'

vi.mock('../lib/api', async () => {
  const { apiMock } = await import('../test/api-stub')
  return apiMock()
})

describe('ConnectionBadge', () => {
  it('should render a connected label with an ok tone and a pulsing dot', () => {
    // Arrange & Act
    const { container } = render(<ConnectionBadge state="connected" />)

    // Assert
    expect(screen.getByText('Connected')).toBeTruthy()
    expect((container.firstChild as HTMLElement).className).toContain('badge-ok')
    expect(container.querySelector('.dot.pulse')).toBeTruthy()
  })

  it('should render a disconnected label with a muted tone and no pulse', () => {
    // Arrange & Act
    const { container } = render(<ConnectionBadge state="disconnected" />)

    // Assert
    expect(screen.getByText('Disconnected')).toBeTruthy()
    expect((container.firstChild as HTMLElement).className).toContain('badge-muted')
    expect(container.querySelector('.dot.pulse')).toBeNull()
  })

  it('should render the error state with a danger tone, distinct from disconnected', () => {
    // Arrange & Act
    const { container } = render(<ConnectionBadge state="error" />)

    // Assert
    expect(screen.getByText('Error')).toBeTruthy()
    expect((container.firstChild as HTMLElement).className).toContain('badge-danger')
  })
})

describe('AppShell', () => {
  beforeEach(() => {
    resetApi()
  })

  it('should render every navigation link, the children and the connected status', async () => {
    // Arrange & Act
    renderWithSnapshot(
      <AppShell>
        <div>page body</div>
      </AppShell>,
      representativeSnapshot(),
    )

    // Assert
    expect(await screen.findByText('Connected')).toBeTruthy()
    for (const label of ['Dashboard', 'Slots', 'Performance', 'Hardware', 'Settings']) {
      expect(screen.getByRole('link', { name: label })).toBeTruthy()
    }
    expect(screen.getByText('page body')).toBeTruthy()
    expect(screen.getByText('http://localhost:8080')).toBeTruthy()
  })

  it('should fall back to a disconnected status when no snapshot is available', async () => {
    // Arrange & Act
    renderWithSnapshot(
      <AppShell>
        <div>body</div>
      </AppShell>,
      emptySnapshot(),
    )

    // Assert
    expect(await screen.findByText('Disconnected')).toBeTruthy()
  })
})
