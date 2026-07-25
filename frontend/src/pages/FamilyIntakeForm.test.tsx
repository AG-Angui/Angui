import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { IntakeDraft, IntakeSession, SubmitIntakeAnswerResponse } from '../api/intake'
import { FamilyIntakeForm } from './FamilyIntakeForm'

const mocked = vi.hoisted(() => ({
  confirmIntakeSession: vi.fn(),
  createIntakeSession: vi.fn(),
  getIntakeDraft: vi.fn(),
  submitIntakeAnswer: vi.fn(),
}))

vi.mock('../auth/useAuth', () => ({
  useAuth: () => ({ token: 'family-session', user: { id: 'family-1' } }),
}))

vi.mock('../api/intake', () => ({
  confirmIntakeSession: (...args: unknown[]) => mocked.confirmIntakeSession(...args),
  createIntakeSession: (...args: unknown[]) => mocked.createIntakeSession(...args),
  getIntakeDraft: (...args: unknown[]) => mocked.getIntakeDraft(...args),
  submitIntakeAnswer: (...args: unknown[]) => mocked.submitIntakeAnswer(...args),
}))

const collectingSession: IntakeSession = {
  id: 'intake-1',
  status: 'collecting',
  missing_fields: ['last_seen'],
  phase: 'phase_one',
  completed_phase_one_fields: ['basic_information'],
  missing_phase_one_fields: ['last_seen'],
  phase_transition_ready: false,
  next_question: { field: 'last_seen', prompt: '请描述最后出现的地点和时间', required: true },
  guidance_mode: 'rule_based',
  privacy_notice: '仅用于本次问询。',
}

const readySession: IntakeSession = {
  ...collectingSession,
  status: 'ready_for_confirmation',
  completed_phase_one_fields: ['basic_information', 'health_status', 'behavior_habits', 'last_seen'],
  missing_phase_one_fields: [],
  phase_transition_ready: true,
  next_question: null,
}

const profileDraft: IntakeDraft = {
  status: 'draft',
  source_scope: 'family_provided intake answers from this session only',
  generated_at: '2026-07-25T08:00:00Z',
  requires_human_confirmation: true,
  profile: {
    physical_description: '佩戴眼镜，穿蓝色外套。',
    clothing_description: null,
    health_notes: '行动较慢，需要留意。',
    mobility_notes: '行动较慢，需要留意。',
    transportation_ability: null,
    frequent_locations: null,
    last_seen_information: '模拟社区北门',
    behavior_habits: null,
    suspicious_motive: null,
  },
  field_metadata: [
    { field: 'physical_description', source_field: 'basic_information', source: 'family_provided', status: 'draft', generated_at: '2026-07-25T08:00:00Z' },
    { field: 'health_notes', source_field: 'health_status', source: 'family_provided', status: 'draft', generated_at: '2026-07-25T08:01:00Z' },
    { field: 'last_seen_information', source_field: 'last_seen', source: 'family_provided', status: 'draft', generated_at: '2026-07-25T08:02:00Z' },
  ],
  missing_fields: [],
  assessments: [],
  confirmation_blocked_reasons: [],
  direction_hypotheses: [],
}

function answerResponse(session: IntakeSession): SubmitIntakeAnswerResponse {
  return {
    ...session,
    raw_answer: '模拟社区北门',
    candidate_fields: [{
      field: 'last_seen',
      value: '模拟社区北门',
      source: 'family_provided',
      status: 'draft',
      generated_at: '2026-07-25T08:02:00Z',
      model: null,
      template_version: null,
      source_text: '模拟社区北门',
      confidence: null,
    }],
    assessments: [],
  }
}

describe('FamilyIntakeForm', () => {
  beforeEach(() => {
    window.sessionStorage.clear()
    vi.clearAllMocks()
  })

  it('restores the current-tab draft and does not create a second intake session', async () => {
    window.sessionStorage.setItem('angui:intake-tab-draft:family-1', JSON.stringify({ session: collectingSession, answer: '尚未提交的地点描述' }))

    render(<FamilyIntakeForm onCancel={vi.fn()} onConfirmed={vi.fn().mockResolvedValue(undefined)} />)

    expect(await screen.findByRole('heading', { name: '最后出现情况' })).toBeInTheDocument()
    expect(screen.getByDisplayValue('尚未提交的地点描述')).toBeInTheDocument()
    expect(mocked.createIntakeSession).not.toHaveBeenCalled()
  })

  it('discards a malformed stored session before rendering the intake flow', async () => {
    const malformedSession = { ...collectingSession, missing_phase_one_fields: undefined }
    window.sessionStorage.setItem('angui:intake-tab-draft:family-1', JSON.stringify({ session: malformedSession, answer: 'stale answer' }))

    render(<FamilyIntakeForm onCancel={vi.fn()} onConfirmed={vi.fn().mockResolvedValue(undefined)} />)

    expect(await screen.findByRole('button', { name: '开始问询' })).toBeInTheDocument()
    expect(window.sessionStorage.getItem('angui:intake-tab-draft:family-1')).toBeNull()
  })

  it('shows field-level provenance and sends a replacement when the family corrects a draft answer', async () => {
    mocked.createIntakeSession.mockResolvedValue(collectingSession)
    mocked.submitIntakeAnswer.mockResolvedValue(answerResponse(readySession))
    mocked.getIntakeDraft.mockResolvedValue(profileDraft)

    render(<FamilyIntakeForm onCancel={vi.fn()} onConfirmed={vi.fn().mockResolvedValue(undefined)} />)

    fireEvent.click(screen.getByRole('button', { name: '开始问询' }))
    await screen.findByRole('heading', { name: '最后出现情况' })
    fireEvent.change(screen.getByRole('textbox', { name: /请描述最后出现的地点和时间/ }), { target: { value: '模拟社区北门' } })
    fireEvent.click(screen.getByRole('button', { name: '保存并继续' }))

    expect(await screen.findByText('问询整理出的画像草稿')).toBeInTheDocument()
    expect(screen.getByText('来源：家属提供 · 健康情况')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '修改健康情况' }))
    const editInput = screen.getByRole('textbox', { name: '修订后的家属回答' })
    fireEvent.change(editInput, { target: { value: '家属已核对：行动不受限。' } })
    fireEvent.click(screen.getByRole('button', { name: '保存修订并刷新草稿' }))

    await waitFor(() => expect(mocked.submitIntakeAnswer).toHaveBeenLastCalledWith(
      'family-session',
      'intake-1',
      { field: 'health_status', answer: '家属已核对：行动不受限。', replace: true },
    ))
  })

  it('requires an explicit second confirmation before it creates a case', async () => {
    window.sessionStorage.setItem('angui:intake-tab-draft:family-1', JSON.stringify({ session: readySession, answer: '' }))
    mocked.getIntakeDraft.mockResolvedValue(profileDraft)
    mocked.confirmIntakeSession.mockResolvedValue({ case_id: 'case-1', case_code: 'AG-0001', status: 'active', confirmation_status: 'human_confirmed', confirmed_at: '2026-07-25T08:10:00Z' })
    const onConfirmed = vi.fn().mockResolvedValue(undefined)

    render(<FamilyIntakeForm onCancel={vi.fn()} onConfirmed={onConfirmed} />)

    await screen.findByText('确认后写入案件的资料')
    fireEvent.change(screen.getByRole('textbox', { name: '姓名或称呼' }), { target: { value: '模拟老人' } })
    fireEvent.click(screen.getByRole('button', { name: '人工确认并创建案件' }))

    expect(mocked.confirmIntakeSession).not.toHaveBeenCalled()
    const confirmationDialog = screen.getByRole('alertdialog')
    expect(confirmationDialog).toHaveFocus()
    expect(screen.getByRole('button', { name: '请完成二次确认' })).toBeDisabled()

    fireEvent.click(screen.getByRole('button', { name: '确认并创建案件' }))
    await waitFor(() => expect(mocked.confirmIntakeSession).toHaveBeenCalledTimes(1))
    expect(onConfirmed).toHaveBeenCalledWith('case-1', 'AG-0001')
  })
})
