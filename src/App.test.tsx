// Integration smoke test: the real App wires BrowserRouter + SnapshotProvider +
// routes. Rendering App (rather than a page in isolation) exercises the routing
// and provider composition end to end against the mocked Tauri bridge.
import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import { api, resetApi } from './test/api-stub'
import { representativeSnapshot } from './test/fixtures'

vi.mock('./lib/api', async () => {
  const { apiMock } = await import('./test/api-stub')
  return apiMock()
})

describe('App', () => {
  beforeEach(() => {
    resetApi()
    window.history.pushState({}, '', '/')
  })

  it('should render the dashboard with navigation at the root route', async () => {
    // Arrange
    api.getSnapshot.mockResolvedValue(representativeSnapshot())

    // Act
    render(<App />)

    // Assert
    expect(await screen.findByRole('heading', { name: 'Connection' })).toBeTruthy()
    expect(screen.getByRole('link', { name: 'Slots' })).toBeTruthy()
  })

  it('should route to the slots page at /slots', async () => {
    // Arrange
    window.history.pushState({}, '', '/slots')
    api.getSnapshot.mockResolvedValue(representativeSnapshot())

    // Act
    render(<App />)

    // Assert
    expect(await screen.findAllByRole('heading', { name: /^Slot \d/ })).toHaveLength(3)
  })
})
