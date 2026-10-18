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
    const url = await screen.findByLabelText('Server URL')

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
    const url = await screen.findByLabelText('Server URL')

    // Act
    await userEvent.clear(url)
    await userEvent.type(url, 'not a url')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    // Assert
    const errorMessages = await screen.findAllByText('Enter a valid URL')
    expect(errorMessages.length).toBeGreaterThan(0)
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
    await screen.findByLabelText('Server URL')

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
    await screen.findByLabelText('Server URL')

    // Act
    await userEvent.click(screen.getByRole('button', { name: 'Prune history now' }))

    // Assert
    expect(await screen.findByText('Pruned 5 rows.')).toBeTruthy()
  })

  it('should persist the active profile, its URL, and all profiles when switching and saving', async () => {
    // Arrange
    api.updateSettings.mockImplementation((config) => Promise.resolve(config))
    renderWithSnapshot(<Settings />, makeSnapshot())
    await screen.findByLabelText('Server URL')

    // Add a second profile
    await userEvent.click(screen.getByRole('button', { name: 'Add profile' }))

    // Switch the active profile to the new one
    const activeSelect = await screen.findByLabelText('Active profile')
    await userEvent.selectOptions(activeSelect, 'Profile 1')

    // The top Server URL field now edits the active profile's URL
    const url = await screen.findByLabelText('Server URL')
    await userEvent.clear(url)
    await userEvent.type(url, 'http://example.test:9999')

    // Act
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))

    // Assert
    await waitFor(() => expect(api.updateSettings).toHaveBeenCalledTimes(1))
    expect(api.updateSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        serverUrl: 'http://example.test:9999',
        activeProfile: 'Profile 1',
        profiles: expect.arrayContaining([
          { name: 'Default', serverUrl: 'http://localhost:8080' },
          { name: 'Profile 1', serverUrl: 'http://example.test:9999' },
        ]),
      }),
    )
    expect(await screen.findByText('Settings saved.')).toBeTruthy()
  })
})
