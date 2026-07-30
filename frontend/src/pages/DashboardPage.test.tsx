import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { DashboardPage } from './DashboardPage'

const mocked = vi.hoisted(() => ({
  getCase: vi.fn(),
  listCases: vi.fn(),
  listCommandIntake: vi.fn(),
  globalCapabilities: [] as string[],
}))

vi.mock('../auth/useAuth', () => ({
  useAuth: () => ({
    token: 'test-session',
    user: { id: 'family-1', email: 'family@demo.invalid', display_name: '模拟家属', account_type: 'member', global_capabilities: mocked.globalCapabilities },
  }),
}))
vi.mock('../api/cases', () => ({
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  listCommandIntake: (...args: unknown[]) => mocked.listCommandIntake(...args),
}))
vi.mock('../components/ServiceStatus', () => ({ ServiceStatus: () => <span>服务状态</span> }))

describe('DashboardPage', () => {
  it('warns when the 20-detail statistics limit truncates the visible case list', async () => {
    mocked.listCases.mockResolvedValue(
      Array.from({ length: 21 }, (_, index) => ({
        id: `case-${index + 1}`,
        case_code: `AG-${index + 1}`,
        status: 'active',
        access_role: 'family',
        display_name: `模拟案件 ${index + 1}`,
        last_seen_at: null,
        last_seen_location: null,
        created_at: '2026-07-24T00:00:00Z',
        updated_at: '2026-07-24T00:00:00Z',
      })),
    )
    mocked.getCase.mockResolvedValue({ clues: [] })

    render(<DashboardPage />)

    expect(await screen.findByText('部分案件详情暂时不可用，统计数据可能不完整。')).toBeInTheDocument()
    expect(mocked.getCase).toHaveBeenCalledTimes(20)
  })

  it('shows the commander intake queue in overview metrics and real-time status', async () => {
    mocked.globalCapabilities = ['commander']
    mocked.listCases.mockResolvedValue([])
    mocked.listCommandIntake.mockResolvedValue([{
      id: 'pending-case',
      case_code: 'AG-PENDING',
      created_at: '2026-07-24T00:00:00Z',
      last_seen_at: '2026-07-24T08:30:00Z',
      area_hint: '北门区域',
      elder_age: 76,
    }])

    render(<DashboardPage />)

    expect(await screen.findByText('待受理案件')).toBeInTheDocument()
    expect(screen.getByText('AG-PENDING')).toBeInTheDocument()
    expect(screen.getByText(/地区：北门区域.*走失时间：2026-07-24T08:30:00Z.*老人年龄：76 岁/)).toBeInTheDocument()
    expect(screen.getByText('待受理')).toBeInTheDocument()
    expect(mocked.listCommandIntake).toHaveBeenCalledWith('test-session')
  })
})
