import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { apiRequest } from '../api/client'
import { AuthProvider } from './AuthContext'
import { useAuth } from './useAuth'

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function AuthProbe() {
  const { login, logout, user } = useAuth()
  return (
    <div>
      <span>{user ? user.email : 'anonymous'}</span>
      <button type="button" onClick={() => void login('family@demo.invalid', 'local-test-input').catch(() => undefined)}>登录</button>
      <button type="button" onClick={() => void logout()}>退出</button>
    </div>
  )
}

describe('AuthProvider', () => {
  it('logs in with the server response and restores a session after refresh', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse(200, {
        token: 'session-created-by-test-server',
        expires_at: '2026-07-24T00:00:00Z',
        user: { id: 'family-1', email: 'family@demo.invalid', display_name: '模拟家属', account_type: 'member', global_capabilities: [] },
      }))
      .mockResolvedValueOnce(jsonResponse(200, {
        id: 'family-1', email: 'family@demo.invalid', display_name: '模拟家属', account_type: 'member', global_capabilities: [],
      }))
      .mockResolvedValueOnce(jsonResponse(200, {
        id: 'family-1', email: 'family@demo.invalid', display_name: '模拟家属', account_type: 'member', global_capabilities: [],
      }))
    vi.stubGlobal('fetch', fetchMock)

    const firstRender = render(<AuthProvider><AuthProbe /></AuthProvider>)
    fireEvent.click(screen.getByRole('button', { name: '登录' }))
    expect(await screen.findByText('family@demo.invalid')).toBeInTheDocument()

    firstRender.unmount()
    render(<AuthProvider><AuthProbe /></AuthProvider>)
    expect(await screen.findByText('family@demo.invalid')).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledTimes(3)
  })

  it('keeps the login screen state on invalid credentials', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(jsonResponse(401, { error: { code: 'unauthorized' } })),
    )
    render(<AuthProvider><AuthProbe /></AuthProvider>)

    fireEvent.click(screen.getByRole('button', { name: '登录' }))
    await waitFor(() => expect(screen.getByText('anonymous')).toBeInTheDocument())
    expect(sessionStorage.getItem('angui.session.token')).toBeNull()
  })

  it('removes session data after logout', async () => {
    sessionStorage.setItem('angui.session.token', 'existing-session')
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse(200, {
        id: 'family-1', email: 'family@demo.invalid', display_name: '模拟家属', account_type: 'member', global_capabilities: [],
      }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetchMock)
    render(<AuthProvider><AuthProbe /></AuthProvider>)

    expect(await screen.findByText('family@demo.invalid')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: '退出' }))
    expect(await screen.findByText('anonymous')).toBeInTheDocument()
    expect(sessionStorage.getItem('angui.session.token')).toBeNull()
  })

  it('removes stale session data when an authenticated request receives 401', async () => {
    sessionStorage.setItem('angui.session.token', 'existing-session')
    vi.stubGlobal(
      'fetch',
      vi.fn()
        .mockResolvedValueOnce(jsonResponse(200, {
          id: 'family-1', email: 'family@demo.invalid', display_name: '模拟家属', account_type: 'member', global_capabilities: [],
        }))
        .mockResolvedValueOnce(jsonResponse(401, { error: { code: 'unauthorized' } })),
    )
    render(<AuthProvider><AuthProbe /></AuthProvider>)

    expect(await screen.findByText('family@demo.invalid')).toBeInTheDocument()
    await expect(apiRequest('/cases', {}, 'existing-session')).rejects.toMatchObject({ status: 401 })
    await waitFor(() => expect(screen.getByText('anonymous')).toBeInTheDocument())
    expect(sessionStorage.getItem('angui.session.token')).toBeNull()
  })
})
