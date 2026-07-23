import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import type { AuthContextValue } from './auth/auth-context'

const mocked = vi.hoisted(() => ({
  auth: null as AuthContextValue | null,
  listCases: vi.fn().mockResolvedValue([]),
  getCase: vi.fn(),
}))

vi.mock('./auth/useAuth', () => ({ useAuth: () => mocked.auth }))
vi.mock('./api/cases', () => ({
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  createCase: vi.fn(),
  createClue: vi.fn(),
  addCaseMember: vi.fn(),
  reviewClue: vi.fn(),
  updateCaseStatus: vi.fn(),
}))
vi.mock('./components/ServiceStatus', () => ({ ServiceStatus: () => <span>服务状态</span> }))

function setAuth(role: NonNullable<AuthContextValue['user']>['role'] | null) {
  mocked.auth = {
    token: role ? 'test-session' : null,
    user: role ? { id: `${role}-1`, email: `${role}@demo.invalid`, display_name: '模拟用户', role } : null,
    isLoading: false,
    login: vi.fn(),
    logout: vi.fn(),
  }
}

function renderApp(path = '/') {
  return render(<MemoryRouter initialEntries={[path]}><App /></MemoryRouter>)
}

describe('application role routing', () => {
  beforeEach(() => {
    mocked.listCases.mockResolvedValue([])
    mocked.getCase.mockReset()
  })

  it('shows the login page without a session', () => {
    setAuth(null)
    renderApp()
    expect(screen.getByText('账号登录')).toBeInTheDocument()
  })

  it.each([
    ['family', '家属端'],
    ['commander', '指挥端'],
    ['volunteer', '志愿者端'],
  ] as const)('shows only the %s workspace navigation', async (role, workspace) => {
    setAuth(role)
    renderApp()
    await waitFor(() => expect(screen.getByText('行动总览')).toBeInTheDocument())
    expect(screen.getByRole('link', { name: workspace })).toBeInTheDocument()
  })

  it.each([
    ['family', '/command', '指挥端'],
    ['commander', '/volunteer', '志愿者端'],
    ['volunteer', '/family', '家属端'],
  ] as const)('redirects a %s account away from the incompatible %s route', async (role, path, unavailableNavigation) => {
    setAuth(role)
    renderApp(path)
    await waitFor(() => expect(screen.getByText('行动总览')).toBeInTheDocument())
    expect(screen.queryByRole('link', { name: unavailableNavigation })).not.toBeInTheDocument()
  })

  it.each(['learner', 'admin'] as const)('does not imply case access for %s accounts', async (role) => {
    setAuth(role)
    renderApp()
    expect(await screen.findByText(role === 'learner' ? '新人账号暂未获得案件权限' : '管理员账号不自动拥有案件权限')).toBeInTheDocument()
  })
})
