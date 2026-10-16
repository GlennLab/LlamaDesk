import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Settings from './Settings'
import { api, resetApi } from '../test/api-stub'
import { makeSnapshot } from '../test/fixtures'
import { renderWithSnapshot } from '../test/snapshot-test-utils'
import { DEFAULT_CONFIG } from '../types'

vi.mock('../lib/api', async () => {
  const { apiMock } = await import('../test/api-stub')
  return apiMock()
})

describe('Settings', () => {
  beforeEach(() => {
    resetApi()
  })

  it('should reflect the loaded settings in the form', async () => {
    // Arrange
    api.getSettings.mockResolvedValue({ ...DEFAULT_CONFIG, serverUrl: 'http://example.test:1234' })

    // Act
    renderWithSnapshot(<Settings />, makeSnapshot())

    // Assert
    expect(await screen.findByDisplayValue('http://example.test:1234')).toBeTruthy()
  })

  it('should save edited settings through update_settings', async () => {
    // Arrange
    api.updateSettings.mockImplementation((config) => Promise.resolve(config))
    renderWithSnapshot(<Settings />, makeSnapshot())
    const url = await screen.findByDisplayValue(DEFAULT_CONFIG.serverUrl)

    // Act
    await userEvent.clear(url)
    await userEvent.type(url, 'http://example.test:9999')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    // Assert
    await waitFor(() => expect(api.updateSettings).toHaveBeenCalledTimes(1))
    expect(api.updateSettings).toHaveBeenCalledWith(
      expect.objectContaining({ serverUrl: 'http://example.test:9999' }),
    )
    expect(await screen.findByText('Settings saved.')).toBeTruthy()
  })

  it('should block saving and show a field error for an invalid URL', async () => {
    // Arrange
    renderWithSnapshot(<Settings />, makeSnapshot())
    const url = await screen.findByDisplayValue(DEFAULT_CONFIG.serverUrl)

    // Act
    await userEvent.clear(url)
    await userEvent.type(url, 'not a url')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    // Assert
    expect(await screen.findByText('Enter a valid URL')).toBeTruthy()
    expect(api.updateSettings).not.toHaveBeenCalled()
  })

  it('should show the connection test result after probing the server', async () => {
    // Arrange
    api.testConnection.mockResolvedValue({
      ok: true,
      message: 'reachable',
      role: 'router',
      buildInfo: 'b1',
      modelCount: 3,
    })
    renderWithSnapshot(<Settings />, makeSnapshot())
    await screen.findByDisplayValue(DEFAULT_CONFIG.serverUrl)

    // Act
    await userEvent.click(screen.getByRole('button', { name: 'Test connection' }))

    // Assert
    expect(await screen.findByText('reachable')).toBeTruthy()
    expect(screen.getByText('ok')).toBeTruthy()
    expect(api.testConnection).toHaveBeenCalledWith(DEFAULT_CONFIG.serverUrl)
  })

  it('should report the number of rows removed when pruning history', async () => {
    // Arrange
    api.pruneHistory.mockResolvedValue(5)
    renderWithSnapshot(<Settings />, makeSnapshot())
    await screen.findByDisplayValue(DEFAULT_CONFIG.serverUrl)

    // Act
    await userEvent.click(screen.getByRole('button', { name: 'Prune history now' }))

    // Assert
    expect(await screen.findByText('Pruned 5 rows.')).toBeTruthy()
  })
})
