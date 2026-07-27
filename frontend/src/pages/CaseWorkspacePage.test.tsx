import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { CaseDetail, CaseRole, CaseStatus } from '../api/cases'
import { CaseWorkspacePage } from './CaseWorkspacePage'

const mocked = vi.hoisted(() => ({
  getCase: vi.fn(),
  getCaseResourceConfiguration: vi.fn(),
  listCases: vi.fn(),
  listCaseClues: vi.fn(),
  addCaseMember: vi.fn(),
  reviewClue: vi.fn(),
}))

vi.mock('../auth/useAuth', () => ({
  useAuth: () => ({ token: 'test-session' }),
}))
vi.mock('../api/cases', () => ({
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  getCaseResourceConfiguration: (...args: unknown[]) => mocked.getCaseResourceConfiguration(...args),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  listCaseClues: (...args: unknown[]) => mocked.listCaseClues(...args),
  addCaseMember: (...args: unknown[]) => mocked.addCaseMember(...args),
  createCase: vi.fn(),
  createClue: vi.fn(),
  reviewClue: (...args: unknown[]) => mocked.reviewClue(...args),
  createCasePlace: vi.fn(),
  uploadCaseAttachment: vi.fn(),
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

function detail(
  id: string,
  displayName: string,
  accessRole: CaseRole = 'family',
  status: CaseStatus = 'active',
): CaseDetail {
  return {
    id,
    case_code: `AG-${id}`,
    status,
    access_role: accessRole,
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
    places: [],
    attachments: [],
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
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ['frequent'],
    })

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

  it('lets an authorized commander invite the demo volunteer to an active case', async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: 'case-command', case_code: 'AG-COMMAND', status: 'active', access_role: 'commander', display_name: '指挥案件',
        last_seen_at: null, last_seen_location: null, created_at: '2026-07-24T00:00:00Z', updated_at: '2026-07-24T00:00:00Z',
      },
    ])
    mocked.getCase.mockResolvedValue(detail('case-command', '指挥案件', 'commander'))
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ['frequent'],
    })
    mocked.addCaseMember.mockResolvedValue({})

    render(<CaseWorkspacePage mode="commander" />)

    await screen.findByRole('heading', { name: '指挥案件' })
    fireEvent.change(screen.getByPlaceholderText('成员邮箱'), { target: { value: 'volunteer@demo.invalid' } })
    fireEvent.click(screen.getByRole('button', { name: '添加案件成员' }))

    await waitFor(() => expect(mocked.addCaseMember).toHaveBeenCalledWith(
      'test-session',
      'case-command',
      'volunteer@demo.invalid',
      'volunteer',
    ))
  })

  it('does not expose closed-case controls that create supplementary information', async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: 'case-closed', case_code: 'AG-CLOSED', status: 'closed', access_role: 'commander', display_name: '已关闭案件',
        last_seen_at: null, last_seen_location: null, created_at: '2026-07-24T00:00:00Z', updated_at: '2026-07-24T00:00:00Z',
      },
    ])
    mocked.getCase.mockResolvedValue(detail('case-closed', '已关闭案件', 'commander', 'closed'))
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ['frequent'],
    })

    render(<CaseWorkspacePage mode="commander" />)

    await screen.findByRole('heading', { name: '已关闭案件' })
    expect(screen.queryByRole('button', { name: '提交线索' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '提交地点' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: '上传图片' })).not.toBeInTheDocument()
    expect(screen.getByRole('combobox', { name: '案件状态' })).toBeDisabled()
  })

  it('loads a filtered commander queue and requires confirmation before reviewing a clue', async () => {
    vi.clearAllMocks()
    const pendingClue = {
      id: 'clue-pending', case_id: 'case-command', status: 'pending_review', source: 'field responder', source_type: 'field_report',
      content: 'A fictional field observation.', raw_record_reference: null, occurred_at: null, reported_at: '2026-07-24T00:00:00Z', confirmed_at: null,
      location_text: null, location_precision: null, next_action: null, linked_task_reference: null, related_clue_id: null, relationship_type: null,
      review_reason: null, attachment_ids: [], created_at: '2026-07-24T00:00:00Z', updated_at: '2026-07-24T00:00:00Z', reviewed_at: null, is_own_submission: false,
    }
    mocked.listCases.mockResolvedValue([{ id: 'case-command', case_code: 'AG-COMMAND', status: 'active', access_role: 'commander', display_name: '指挥案件', last_seen_at: null, last_seen_location: null, created_at: '2026-07-24T00:00:00Z', updated_at: '2026-07-24T00:00:00Z' }])
    mocked.getCase.mockResolvedValue(detail('case-command', '指挥案件', 'commander'))
    mocked.getCaseResourceConfiguration.mockResolvedValue({ attachment_max_image_bytes: 5 * 1024 * 1024, attachment_max_per_case: 12, case_place_types: ['frequent'] })
    mocked.listCaseClues.mockResolvedValue({ items: [pendingClue], page: 1, page_size: 25, total: 1 })
    mocked.reviewClue.mockResolvedValue({ ...pendingClue, status: 'confirmed' })

    render(<CaseWorkspacePage mode="commander" />)

    await screen.findByRole('heading', { name: '指挥案件' })
    await waitFor(() => expect(mocked.listCaseClues).toHaveBeenCalledWith('test-session', 'case-command', expect.objectContaining({ status: 'pending_review', sort: 'created_at', order: 'desc' })))
    fireEvent.change(screen.getByLabelText('来源类型筛选'), { target: { value: 'field_report' } })
    await waitFor(() => expect(mocked.listCaseClues).toHaveBeenLastCalledWith('test-session', 'case-command', expect.objectContaining({ source_type: 'field_report' })))

    fireEvent.change(screen.getByLabelText('审核理由'), { target: { value: 'Reviewed against the fictional record.' } })
    fireEvent.click(screen.getByRole('button', { name: '确认' }))
    expect(mocked.reviewClue).not.toHaveBeenCalled()
    expect(screen.getByRole('dialog', { name: '确认审核操作' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '确认提交' }))
    await waitFor(() => expect(mocked.reviewClue).toHaveBeenCalledWith('test-session', 'clue-pending', expect.objectContaining({ status: 'confirmed', reason: 'Reviewed against the fictional record.' })))
  })
})
