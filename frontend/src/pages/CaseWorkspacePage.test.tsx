import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { CaseDetail } from '../api/cases'
import { CaseWorkspacePage } from './CaseWorkspacePage'

const mocked = vi.hoisted(() => ({
  getCase: vi.fn(),
  listCases: vi.fn(),
}))

vi.mock('../auth/useAuth', () => ({
  useAuth: () => ({ token: 'test-session' }),
}))
vi.mock('../api/cases', () => ({
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  addCaseMember: vi.fn(),
  createCase: vi.fn(),
  createClue: vi.fn(),
  reviewClue: vi.fn(),
  updateCaseStatus: vi.fn(),
}))

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve
    reject = nextReject
  })
  return { promise, resolve, reject }
}

function detail(id: string, displayName: string): CaseDetail {
  return {
    id,
    case_code: `AG-${id}`,
    status: 'active',
    access_role: 'family',
    elder_profile: {
      id: `profile-${id}`,
      display_name: displayName,
      age: null,
      gender: null,
      physical_description: null,
      clothing_description: null,
      health_notes: null,
      last_seen_at: null,
      last_seen_location: null,
    },
    clues: [],
    created_at: '2026-07-24T00:00:00Z',
    updated_at: '2026-07-24T00:00:00Z',
  }
}

describe('CaseWorkspacePage', () => {
  it('does not let an earlier detail request overwrite the most recently selected case', async () => {
    const firstRequest = deferred<CaseDetail>()
    const secondRequest = deferred<CaseDetail>()
    mocked.listCases.mockResolvedValue([
      {
        id: 'case-1', case_code: 'AG-1', status: 'active', access_role: 'family', display_name: '案件甲',
        last_seen_at: null, last_seen_location: null, created_at: '2026-07-24T00:00:00Z', updated_at: '2026-07-24T00:00:00Z',
      },
      {
        id: 'case-2', case_code: 'AG-2', status: 'active', access_role: 'family', display_name: '案件乙',
        last_seen_at: null, last_seen_location: null, created_at: '2026-07-24T00:00:00Z', updated_at: '2026-07-24T00:00:00Z',
      },
    ])
    mocked.getCase.mockImplementation((_token: string, caseId: string) => (
      caseId === 'case-1' ? firstRequest.promise : secondRequest.promise
    ))

    render(<CaseWorkspacePage mode="family" />)
    await waitFor(() => expect(mocked.getCase).toHaveBeenCalledWith('test-session', 'case-1'))

    fireEvent.click(screen.getByText('案件乙'))
    await waitFor(() => expect(mocked.getCase).toHaveBeenCalledWith('test-session', 'case-2'))

    await act(async () => {
      secondRequest.resolve(detail('case-2', '最新案件详情'))
      await secondRequest.promise
    })
    expect(screen.getByRole('heading', { name: '最新案件详情' })).toBeInTheDocument()

    await act(async () => {
      firstRequest.reject(new Error('过期请求'))
      await firstRequest.promise.catch(() => undefined)
    })
    expect(screen.getByRole('heading', { name: '最新案件详情' })).toBeInTheDocument()
    expect(screen.queryByText('过期请求')).not.toBeInTheDocument()
  })
})
