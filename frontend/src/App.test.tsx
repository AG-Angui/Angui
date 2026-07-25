import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
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

function setAuth(accountType: NonNullable<AuthContextValue['user']>['account_type'] | null) {
  mocked.auth = {
    token: accountType ? 'test-session' : null,
    user: accountType
      ? { id: `${accountType}-1`, email: `${accountType}@demo.invalid`, display_name: '模拟用户', account_type: accountType, global_capabilities: [] }
      : null,
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

  it.each(['member'] as const)(
    'shows all case workspaces to the operational %s account',
    async (role) => {
    setAuth(role)
    renderApp()
    await waitFor(() => expect(screen.getByText('行动总览')).toBeInTheDocument())
    for (const workspace of ['家属端', '指挥端', '志愿者端']) {
      expect(screen.getByRole('link', { name: workspace })).toBeInTheDocument()
    }
    },
  )

  it.each([
    ['member', '/command', '指挥端'],
    ['member', '/volunteer', '志愿者端'],
    ['member', '/family', '家属端'],
  ] as const)('allows a %s account to open the %s route when its case membership grants access', async (role, path, workspace) => {
    setAuth(role)
    renderApp(path)
    expect(await screen.findByRole('link', { name: workspace })).toBeInTheDocument()
  })

  it.each(['learner'] as const)('does not imply case access for %s accounts', async (role) => {
    setAuth(role)
    renderApp()
    expect(await screen.findByText('新人账号暂未获得案件权限')).toBeInTheDocument()
  })
})
