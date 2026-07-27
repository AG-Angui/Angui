import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import type { AuthContextValue } from './auth/auth-context'
import type { GlobalCapability } from './api/auth'

const mocked = vi.hoisted(() => ({
  auth: null as AuthContextValue | null,
  listCases: vi.fn().mockResolvedValue([]),
  getCase: vi.fn(),
  getCaseResourceConfiguration: vi.fn(),
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
  updateElderProfile: vi.fn(),
  getCaseResourceConfiguration: (...args: unknown[]) => mocked.getCaseResourceConfiguration(...args),
}))
vi.mock('./components/ServiceStatus', () => ({ ServiceStatus: () => <span>服务状态</span> }))

function setAuth(
  accountType: NonNullable<AuthContextValue['user']>['account_type'] | null,
  globalCapabilities: readonly GlobalCapability[] = [],
) {
  mocked.auth = {
    token: accountType ? 'test-session' : null,
    user: accountType
      ? { id: `${accountType}-1`, email: `${accountType}@demo.invalid`, display_name: '模拟用户', account_type: accountType, global_capabilities: [...globalCapabilities] }
      : null,
    isLoading: false,
    isLoggingOut: false,
    sessionNotice: null,
    login: vi.fn(),
    logout: vi.fn(),
    refreshUser: vi.fn(),
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

  it('shows workspaces that match the operational account capabilities', async () => {
    setAuth('member', ['commander', 'volunteer'])
    renderApp()
    await waitFor(() => expect(screen.getByText('行动总览')).toBeInTheDocument())
    for (const workspace of ['家属端', '指挥端', '志愿者端']) {
      expect(screen.getByRole('link', { name: workspace })).toBeInTheDocument()
    }
  })

  it.each([
    ['/command', ['commander']],
    ['/volunteer', ['volunteer']],
    ['/family', []],
  ] as const)('allows an operational account with %s capability to open the workspace route', async (path, capabilities) => {
    setAuth('member', capabilities)
    renderApp(path)
    expect(await screen.findByRole('region', { name: '案件列表' })).toBeInTheDocument()
  })

  it.each([
    ['family account', [], '/command'],
    ['volunteer account', ['volunteer'], '/command'],
  ] as const)('redirects a %s away from the command workspace', async (_description, capabilities, path) => {
    setAuth('member', capabilities)
    renderApp(path)
    expect(await screen.findByText('行动总览')).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: '指挥端' })).not.toBeInTheDocument()
  })

  it.each(['learner'] as const)('does not imply case access for %s accounts', async (role) => {
    setAuth(role)
    renderApp()
    expect(await screen.findByText('新人账号暂未获得案件权限')).toBeInTheDocument()
  })
})
