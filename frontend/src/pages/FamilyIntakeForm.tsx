import { Button, Chip, Input, Spinner, TextArea } from '@heroui/react'
import { AlertTriangle, ArrowLeft, CheckCircle2, CircleHelp, FilePenLine, ShieldCheck } from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ApiClientError } from '../api/client'
import {
  confirmIntakeSession,
  createIntakeSession,
  getIntakeDraft,
  submitIntakeAnswer,
} from '../api/intake'
import type {
  ConfirmedIntakeProfile,
  IntakeAssessment,
  IntakeDraft,
  IntakeDraftProfile,
  IntakeProfileDraftFieldMetadata,
  IntakeSession,
} from '../api/intake'
import { useAuth } from '../auth/useAuth'

const questionLabels: Record<string, string> = {
  basic_information: '基本信息',
  health_status: '健康情况',
  behavior_habits: '行为习惯',
  last_seen: '最后出现情况',
  frequent_locations: '常去地点',
  suspicious_motive: '可疑动机',
  belongings: '随身物品与衣着',
  transport_ability: '出行能力',
  follow_up_clues: '后续线索',
}

const questionReasons: Record<string, string> = {
  basic_information: '用于区分被寻找的人，后续仍需要家属逐项核对。',
  health_status: '帮助现场人员理解可能需要的照护与沟通方式。',
  behavior_habits: '帮助判断可能的行动习惯，不会自动成为正式事实。',
  last_seen: '用于建立初始的时间和地点线索。',
  frequent_locations: '可作为待核实的寻找方向，不会直接发布。',
  suspicious_motive: '仅用于记录家属的待核实判断。',
  belongings: '便于后续人工比对衣着和随身物品。',
  transport_ability: '帮助评估可能的行动范围。',
  follow_up_clues: '记录家属尚待核实的补充信息。',
}

const blankProfile: ConfirmedIntakeProfile = {
  display_name: '',
  age: null,
  gender: null,
  physical_description: null,
  clothing_description: null,
  health_notes: null,
  last_seen_at: null,
  last_seen_location: '',
}

type StoredIntakeSession = Pick<
  IntakeSession,
  | 'id'
  | 'status'
  | 'missing_fields'
  | 'phase'
  | 'completed_phase_one_fields'
  | 'missing_phase_one_fields'
  | 'phase_transition_ready'
  | 'next_question'
  | 'guidance_mode'
  | 'privacy_notice'
>

interface StoredIntakeState {
  session: StoredIntakeSession
  answer: string
}

export function FamilyIntakeForm({
  onCancel,
  onConfirmed,
}: {
  onCancel: () => void
  onConfirmed: (caseId: string, caseCode: string) => Promise<void>
}) {
  const { token, user } = useAuth()
  const storageKey = `angui:intake-tab-draft:${user?.id ?? 'anonymous'}`
  const [session, setSession] = useState<IntakeSession | null>(null)
  const [draft, setDraft] = useState<IntakeDraft | null>(null)
  const [answer, setAnswer] = useState('')
  const [profile, setProfile] = useState<ConfirmedIntakeProfile>(blankProfile)
  const [assessments, setAssessments] = useState<IntakeAssessment[]>([])
  const [editSource, setEditSource] = useState<IntakeProfileDraftFieldMetadata | null>(null)
  const [editAnswer, setEditAnswer] = useState('')
  const [confirmReviewOpen, setConfirmReviewOpen] = useState(false)
  const [busyAction, setBusyAction] = useState<'begin' | 'answer' | 'replace' | 'confirm' | null>(null)
  const [error, setError] = useState('')
  const [hasHydrated, setHasHydrated] = useState(false)
  const confirmDialogRef = useRef<HTMLDivElement>(null)

  const isBusy = busyAction !== null
  const displayedAssessments = draft?.assessments ?? assessments
  const sourceOptions = useMemo(() => uniqueSourceOptions(draft), [draft])

  useEffect(() => {
    if (confirmReviewOpen) confirmDialogRef.current?.focus()
  }, [confirmReviewOpen])

  const loadDraft = useCallback(async (sessionId: string, initializeProfile: boolean) => {
    if (!token) return null
    try {
      const nextDraft = await getIntakeDraft(token, sessionId)
      setDraft(nextDraft)
      setAssessments(nextDraft.assessments)
      if (initializeProfile) setProfile(profileFromDraft(nextDraft))
      return nextDraft
    } catch (cause) {
      setError(messageFrom(cause))
      if (cause instanceof ApiClientError && (cause.status === 403 || cause.status === 404)) {
        setDraft(null)
      }
      return null
    }
  }, [token])

  useEffect(() => {
    setHasHydrated(false)
    if (!token || typeof window === 'undefined') {
      setHasHydrated(true)
      return
    }

    const stored = readStoredState(storageKey)
    if (stored) {
      setSession(stored.session)
      setAnswer(stored.answer)
      if (stored.session.status === 'ready_for_confirmation') {
        void loadDraft(stored.session.id, true)
      }
    }
    setHasHydrated(true)
  }, [loadDraft, storageKey, token])

  useEffect(() => {
    if (!hasHydrated || typeof window === 'undefined') return
    if (!session) {
      window.sessionStorage.removeItem(storageKey)
      return
    }
    const stored: StoredIntakeState = {
      session: toStoredSession(session),
      answer,
    }
    window.sessionStorage.setItem(storageKey, JSON.stringify(stored))
  }, [answer, hasHydrated, session, storageKey])

  useEffect(() => {
    if (!answer.trim() || typeof window === 'undefined') return
    const warnBeforeLeave = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warnBeforeLeave)
    return () => window.removeEventListener('beforeunload', warnBeforeLeave)
  }, [answer])

  async function begin() {
    if (!token) return
    setBusyAction('begin')
    setError('')
    try {
      const nextSession = await createIntakeSession(token)
      setSession(nextSession)
      setDraft(null)
      setAssessments([])
      setAnswer('')
    } catch (cause) {
      setError(messageFrom(cause))
    } finally {
      setBusyAction(null)
    }
  }

  async function sendAnswer(field: string, value: string, replace = false) {
    if (!token || !session || !value.trim()) {
      setError('请填写答案，或选择“标记为未知”。')
      return
    }
    setBusyAction(replace ? 'replace' : 'answer')
    setError('')
    try {
      const next = await submitIntakeAnswer(token, session.id, {
        field,
        answer: value.trim(),
        replace,
      })
      setSession(next)
      setAssessments(next.assessments)
      if (replace) {
        const replacedFields = draft?.field_metadata
          .filter((item) => item.source_field === field)
          .map((item) => item.field) ?? []
        setEditSource(null)
        setEditAnswer('')
        const refreshed = await loadDraft(next.id, false)
        if (refreshed) setProfile((current) => syncProfileFields(current, refreshed, replacedFields))
      } else {
        setAnswer('')
        if (next.status === 'ready_for_confirmation') {
          await loadDraft(next.id, true)
        }
      }
    } catch (cause) {
      setError(messageFrom(cause))
      if (cause instanceof ApiClientError && cause.status === 409 && session.status === 'ready_for_confirmation') {
        await loadDraft(session.id, false)
      }
    } finally {
      setBusyAction(null)
    }
  }

  async function submitCurrentAnswer(value = answer) {
    if (!session?.next_question) {
      setError('当前问询状态已变化，请刷新后继续。')
      return
    }
    await sendAnswer(session.next_question.field, value)
  }

  async function confirmCase() {
    if (!token || !session || !draft) return
    if (draft.confirmation_blocked_reasons.length > 0) {
      setError('当前存在阻断性核对项。请返回修改相关问询内容，再重新确认。')
      return
    }
    if (!profile.display_name.trim() || !profile.last_seen_location.trim()) {
      setError('请先确认姓名或称呼，以及最后出现地点。')
      return
    }
    if (!confirmReviewOpen) {
      setConfirmReviewOpen(true)
      return
    }

    setBusyAction('confirm')
    setError('')
    try {
      const response = await confirmIntakeSession(token, session.id, normalizedProfile(profile))
      await onConfirmed(response.case_id, response.case_code)
      clearStoredState(storageKey)
    } catch (cause) {
      setError(messageFrom(cause))
      setConfirmReviewOpen(false)
      if (cause instanceof ApiClientError && cause.status === 409) {
        await loadDraft(session.id, false)
      }
    } finally {
      setBusyAction(null)
    }
  }

  function openSourceEditor(source: IntakeProfileDraftFieldMetadata) {
    const value = draft?.profile[source.field as keyof IntakeDraftProfile] ?? ''
    setEditSource(source)
    setEditAnswer(value ?? '')
    setConfirmReviewOpen(false)
    setError('')
  }

  function requestCancel() {
    if (answer.trim() && typeof window !== 'undefined') {
      const proceed = window.confirm('当前答案尚未提交。它会仅保留在此标签页草稿中；确定暂时离开吗？')
      if (!proceed) return
    }
    onCancel()
  }

  if (!session) {
    return (
      <section className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5" aria-labelledby="intake-start-title">
        <span className="text-xs font-semibold text-brand-700">家属建档 · 规则化问询</span>
        <h2 id="intake-start-title" className="mt-1 text-xl font-bold text-slate-950">先整理信息，再由您确认建案</h2>
        <p className="max-w-2xl text-sm leading-6 text-slate-600">
          问询分阶段进行。家属提供的内容会先作为待确认草稿，系统不会将规则整理结果直接写成案件事实。
        </p>
        <ul className="mt-4 grid gap-2 text-sm text-slate-700 sm:grid-cols-3">
          <li className="rounded-md bg-slate-50 px-3 py-2">1. 分步填写，随时标记未知</li>
          <li className="rounded-md bg-slate-50 px-3 py-2">2. 查看来源与待核对项</li>
          <li className="rounded-md bg-slate-50 px-3 py-2">3. 人工确认后才创建案件</li>
        </ul>
        {error && <Alert>{error}</Alert>}
        <div className="mt-5 flex flex-wrap gap-2">
          <Button variant="primary" onPress={() => void begin()} isDisabled={!hasHydrated || isBusy}>
            {busyAction === 'begin' && <Spinner size="sm" aria-label="正在创建问询" />}
            开始问询
          </Button>
          <Button variant="ghost" onPress={requestCancel} isDisabled={isBusy}>暂不开始</Button>
        </div>
      </section>
    )
  }

  if (draft) {
    return (
      <section className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5" aria-labelledby="intake-draft-title">
        <header className="flex flex-col justify-between gap-3 border-b border-slate-100 pb-4 sm:flex-row sm:items-start">
          <div>
            <span className="text-xs font-semibold text-brand-700">第 3 步 · 人工确认</span>
            <h2 id="intake-draft-title" className="mt-1 text-xl font-bold text-slate-950">核对老人画像草稿</h2>
            <p className="mb-0 mt-1 text-sm leading-6 text-slate-600">每项内容均保持为草稿，只有您完成确认后才会创建正式案件。</p>
          </div>
          <Chip size="sm" variant="soft"><Chip.Label>需要人工确认</Chip.Label></Chip>
        </header>

        <div className="mt-4 rounded-md border border-amber-200 bg-amber-50 px-3 py-3 text-sm leading-6 text-amber-950" role="status">
          <div className="flex items-start gap-2"><ShieldCheck className="mt-0.5 shrink-0" size={17} aria-hidden="true" /><span>以下信息仅来自本次家属问询。请核对来源、时间与内容；未确认前，它们不是正式案件事实。</span></div>
        </div>

        {error && <Alert>{error}</Alert>}
        <AssessmentList items={displayedAssessments} />
        <DraftProfileReview draft={draft} onEditSource={openSourceEditor} />

        {draft.direction_hypotheses.length > 0 && (
          <section className="mt-5" aria-labelledby="direction-hypotheses-title">
            <h3 id="direction-hypotheses-title" className="text-sm font-bold text-slate-950">待核实方向</h3>
            <div className="grid gap-3 md:grid-cols-2">
              {draft.direction_hypotheses.map((item, index) => (
                <article key={`${item.generated_at}-${index}`} className="rounded-md border border-slate-200 bg-slate-50 p-3 text-sm">
                  <strong className="text-slate-900">可能方向（不确定）</strong>
                  <p className="mb-1 mt-2 leading-6 text-slate-700">{item.description}</p>
                  <p className="m-0 text-xs leading-5 text-slate-600">{item.uncertainty_notice}</p>
                </article>
              ))}
            </div>
          </section>
        )}

        {editSource && (
          <section className="mt-5 rounded-md border border-brand-100 bg-brand-50 p-4" aria-labelledby="source-editor-title">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <span className="text-xs font-semibold text-brand-700">返回问询修改</span>
                <h3 id="source-editor-title" className="m-0 text-base font-bold text-slate-950">修改“{questionLabels[editSource.source_field] ?? editSource.source_field}”</h3>
              </div>
              <Button size="sm" variant="ghost" onPress={() => setEditSource(null)} isDisabled={isBusy}>取消修改</Button>
            </div>
            <Field label="修订后的家属回答" required>
              <TextArea value={editAnswer} rows={4} maxLength={2000} onChange={(event) => setEditAnswer(event.target.value)} fullWidth />
            </Field>
            <div className="mt-3 flex flex-wrap gap-2">
              <Button variant="secondary" onPress={() => void sendAnswer(editSource.source_field, editAnswer, true)} isDisabled={isBusy}>
                {busyAction === 'replace' && <Spinner size="sm" aria-label="正在保存修订" />}
                保存修订并刷新草稿
              </Button>
              <Button variant="ghost" onPress={() => void sendAnswer(editSource.source_field, '未知', true)} isDisabled={isBusy}>标记为未知</Button>
            </div>
          </section>
        )}

        {!editSource && (
          <section className="mt-5 border-t border-slate-200 pt-5" aria-labelledby="confirmed-profile-title">
            <div className="flex items-start gap-2">
              <CheckCircle2 className="mt-0.5 shrink-0 text-brand-700" size={18} aria-hidden="true" />
              <div>
                <h3 id="confirmed-profile-title" className="m-0 text-base font-bold text-slate-950">确认后写入案件的资料</h3>
                <p className="mb-0 mt-1 text-sm leading-6 text-slate-600">请在这里修订正式资料。带 * 的两项为当前服务端真正必填内容。</p>
              </div>
            </div>
            <ProfileForm
              profile={profile}
              onChange={(nextProfile) => {
                setProfile(nextProfile)
                setConfirmReviewOpen(false)
              }}
            />
            {confirmReviewOpen && (
              <div ref={confirmDialogRef} tabIndex={-1} className="mt-4 rounded-md border border-brand-100 bg-brand-50 p-4" role="alertdialog" aria-labelledby="confirm-case-title">
                <h4 id="confirm-case-title" className="m-0 text-sm font-bold text-slate-950">确认创建案件？</h4>
                <p className="mb-3 mt-1 text-sm leading-6 text-slate-700">创建后将生成正式案件。问询草稿会保留为本次确认的依据，但不会替代您刚刚核对的资料。</p>
                <div className="flex flex-wrap gap-2">
                  <Button variant="primary" onPress={() => void confirmCase()} isDisabled={isBusy || draft.confirmation_blocked_reasons.length > 0}>
                    {busyAction === 'confirm' && <Spinner size="sm" aria-label="正在创建案件" />}
                    确认并创建案件
                  </Button>
                  <Button variant="ghost" onPress={() => setConfirmReviewOpen(false)} isDisabled={isBusy}>返回编辑</Button>
                </div>
              </div>
            )}
            <div className="mt-5 flex flex-wrap gap-2">
              <Button
                variant="primary"
                onPress={() => void confirmCase()}
                isDisabled={isBusy || confirmReviewOpen || draft.confirmation_blocked_reasons.length > 0}
              >
                {confirmReviewOpen ? '请完成二次确认' : '人工确认并创建案件'}
              </Button>
              <Button variant="ghost" onPress={requestCancel} isDisabled={isBusy}>暂不确认</Button>
            </div>
          </section>
        )}

        {!editSource && sourceOptions.length > 0 && (
          <section className="mt-5 border-t border-slate-200 pt-4">
            <h3 className="m-0 text-sm font-bold text-slate-950">需要补充或修改问询？</h3>
            <p className="mb-3 mt-1 text-xs leading-5 text-slate-600">修订会作为新的草稿答案保存，并刷新当前画像与核对项。</p>
            <div className="flex flex-wrap gap-2">
              {sourceOptions.map((source) => (
                <Button key={source.source_field} size="sm" variant="ghost" onPress={() => openSourceEditor(source)} isDisabled={isBusy}>
                  <FilePenLine size={15} aria-hidden="true" /> 修改{questionLabels[source.source_field] ?? source.source_field}
                </Button>
              ))}
            </div>
          </section>
        )}
      </section>
    )
  }

  const question = session.next_question
  const completed = session.completed_phase_one_fields.length
  const phaseTotal = 4
  const currentLabel = question ? questionLabels[question.field] ?? question.field : '正在整理草稿'

  return (
    <section className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5" aria-labelledby="intake-question-title">
      <header className="border-b border-slate-100 pb-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <span className="text-xs font-semibold text-brand-700">{session.phase === 'phase_one' ? '第 1 步 · 基本情况' : '第 2 步 · 补充线索'}</span>
            <h2 id="intake-question-title" className="mt-1 text-xl font-bold text-slate-950">{currentLabel}</h2>
          </div>
          <Chip size="sm" variant="soft"><Chip.Label>{session.guidance_mode === 'rule_based' ? '规则化问询' : '问询中'}</Chip.Label></Chip>
        </div>
        <div className="mt-4" aria-label="问询进度">
          <div className="flex items-center justify-between text-xs text-slate-600">
            <span>第一阶段已填写 {completed} / {phaseTotal} 项</span>
            <span>{session.phase_transition_ready ? '必要信息已齐全' : `仍缺：${session.missing_phase_one_fields.map(questionLabel).join('、')}`}</span>
          </div>
          <div className="mt-2 h-2 overflow-hidden rounded-full bg-slate-100">
            <div className="h-full rounded-full bg-brand-600 transition-[width] duration-200 motion-reduce:transition-none" style={{ width: `${Math.min(100, (completed / phaseTotal) * 100)}%` }} />
          </div>
        </div>
      </header>

      {error && <Alert>{error}</Alert>}
      <AssessmentList items={displayedAssessments} />

      {question ? (
        <form
          className="mt-5"
          onSubmit={(event) => {
            event.preventDefault()
            void submitCurrentAnswer()
          }}
        >
          <Field label={question.prompt} required={question.required} hint={questionReasons[question.field]}>
            <TextArea value={answer} onChange={(event) => setAnswer(event.target.value)} rows={5} maxLength={2000} fullWidth />
          </Field>
          <p className="mt-2 text-xs leading-5 text-slate-500">仅在当前浏览器标签页暂存尚未提交的答案；不会写入 URL、日志或跨设备存储。</p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button type="submit" variant="primary" isDisabled={isBusy}>
              {busyAction === 'answer' && <Spinner size="sm" aria-label="正在保存答案" />}
              保存并继续
            </Button>
            <Button type="button" variant="ghost" onPress={() => void submitCurrentAnswer('未知')} isDisabled={isBusy}>标记为未知</Button>
            <Button type="button" variant="ghost" onPress={requestCancel} isDisabled={isBusy}><ArrowLeft size={16} aria-hidden="true" />暂时离开</Button>
          </div>
        </form>
      ) : (
        <div className="mt-5 rounded-md border border-slate-200 bg-slate-50 p-4 text-sm text-slate-700">
          <div className="flex items-center gap-2"><Spinner size="sm" /><span>问询已完成，正在获取需要人工确认的画像草稿。</span></div>
          <Button className="mt-3" size="sm" variant="ghost" onPress={() => void loadDraft(session.id, true)} isDisabled={isBusy}>重新获取草稿</Button>
        </div>
      )}
    </section>
  )
}

function DraftProfileReview({ draft, onEditSource }: { draft: IntakeDraft; onEditSource: (source: IntakeProfileDraftFieldMetadata) => void }) {
  const metadata = new Map(draft.field_metadata.map((item) => [item.field, item]))
  const fields: Array<{ field: keyof IntakeDraftProfile; label: string }> = [
    { field: 'physical_description', label: '体貌描述' },
    { field: 'clothing_description', label: '衣着与随身物品' },
    { field: 'health_notes', label: '健康注意事项' },
    { field: 'mobility_notes', label: '行动与移动' },
    { field: 'transportation_ability', label: '出行能力' },
    { field: 'frequent_locations', label: '常去地点' },
    { field: 'last_seen_information', label: '最后出现信息' },
    { field: 'behavior_habits', label: '行为习惯' },
    { field: 'suspicious_motive', label: '可疑动机' },
  ]

  return (
    <section className="mt-5" aria-labelledby="draft-profile-title">
      <h3 id="draft-profile-title" className="m-0 text-base font-bold text-slate-950">问询整理出的画像草稿</h3>
      <div className="mt-3 grid gap-3 md:grid-cols-2">
        {fields.map(({ field, label }) => {
          const value = draft.profile[field]
          const source = metadata.get(field)
          return (
            <article key={field} className="rounded-md border border-slate-200 bg-white p-4">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <h4 className="m-0 text-sm font-bold text-slate-950">{label}</h4>
                <Chip size="sm" variant="soft"><Chip.Label>草稿</Chip.Label></Chip>
              </div>
              <p className="mb-3 mt-3 min-h-12 whitespace-pre-wrap text-sm leading-6 text-slate-700">{value ?? '尚未提供'}</p>
              {source ? (
                <div className="border-t border-slate-100 pt-3 text-xs leading-5 text-slate-600">
                  <span className="block">来源：{source.source === 'family_provided' ? '家属提供' : '待核实提取'} · {questionLabels[source.source_field] ?? source.source_field}</span>
                  <span className="block">生成于：{formatDate(source.generated_at)} · 状态：需人工确认</span>
                  <Button className="mt-2" size="sm" variant="ghost" onPress={() => onEditSource(source)}><FilePenLine size={14} aria-hidden="true" />返回修改</Button>
                </div>
              ) : (
                <p className="m-0 border-t border-slate-100 pt-3 text-xs text-slate-500">暂无可核对的来源记录</p>
              )}
            </article>
          )
        })}
      </div>
    </section>
  )
}

function ProfileForm({ profile, onChange }: { profile: ConfirmedIntakeProfile; onChange: (next: ConfirmedIntakeProfile) => void }) {
  return (
    <div className="mt-4 grid gap-3 sm:grid-cols-2">
      <Field label="姓名或称呼" required><Input value={profile.display_name} maxLength={120} onChange={(event) => onChange({ ...profile, display_name: event.target.value })} fullWidth /></Field>
      <Field label="最后出现地点" required><Input value={profile.last_seen_location} onChange={(event) => onChange({ ...profile, last_seen_location: event.target.value })} fullWidth /></Field>
      <Field label="年龄"><Input type="number" min={0} max={130} value={profile.age ?? ''} onChange={(event) => onChange({ ...profile, age: event.target.value ? Number(event.target.value) : null })} fullWidth /></Field>
      <Field label="性别"><Input value={profile.gender ?? ''} onChange={(event) => onChange({ ...profile, gender: nullable(event.target.value) })} fullWidth /></Field>
      <Field label="最后出现时间"><Input type="datetime-local" value={toDateTimeLocal(profile.last_seen_at)} onChange={(event) => onChange({ ...profile, last_seen_at: event.target.value ? new Date(event.target.value).toISOString() : null })} fullWidth /></Field>
      <div className="hidden sm:block" aria-hidden="true" />
      <Field label="体貌描述"><TextArea value={profile.physical_description ?? ''} onChange={(event) => onChange({ ...profile, physical_description: nullable(event.target.value) })} rows={3} fullWidth /></Field>
      <Field label="衣着描述"><TextArea value={profile.clothing_description ?? ''} onChange={(event) => onChange({ ...profile, clothing_description: nullable(event.target.value) })} rows={3} fullWidth /></Field>
      <div className="sm:col-span-2"><Field label="健康注意事项"><TextArea value={profile.health_notes ?? ''} onChange={(event) => onChange({ ...profile, health_notes: nullable(event.target.value) })} rows={3} fullWidth /></Field></div>
    </div>
  )
}

function AssessmentList({ items }: { items: IntakeAssessment[] }) {
  if (items.length === 0) return null
  return (
    <section className="mt-4 space-y-2" aria-label="规则核对结果">
      {items.map((item, index) => (
        <div key={`${item.field_path}-${item.conflict_type}-${index}`} className={`rounded-md border px-3 py-3 text-sm ${assessmentClass(item.severity)}`}>
          <div className="flex items-start gap-2"><AlertTriangle className="mt-0.5 shrink-0" size={16} aria-hidden="true" /><div><strong>{assessmentLabel(item.severity)}</strong><span className="ml-2">{item.evidence_summary}</span><p className="mb-0 mt-1 text-xs leading-5">{item.suggested_action}</p></div></div>
        </div>
      ))}
    </section>
  )
}

function Field({ label, hint, required, children }: { label: string; hint?: string; required?: boolean; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm font-semibold text-slate-800">{label}{required ? <span aria-hidden="true"> *</span> : null}</span>
      {hint && <span className="mb-2 flex items-start gap-1 text-xs leading-5 text-slate-500"><CircleHelp className="mt-0.5 shrink-0" size={14} aria-hidden="true" />{hint}</span>}
      {children}
    </label>
  )
}

function Alert({ children }: { children: React.ReactNode }) {
  return <div className="mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-3 text-sm leading-6 text-red-800" role="alert">{children}</div>
}

function uniqueSourceOptions(draft: IntakeDraft | null) {
  if (!draft) return []
  const seen = new Set<string>()
  return draft.field_metadata.filter((item) => {
    if (seen.has(item.source_field)) return false
    seen.add(item.source_field)
    return true
  })
}

function profileFromDraft(draft: IntakeDraft): ConfirmedIntakeProfile {
  return {
    ...blankProfile,
    physical_description: draft.profile.physical_description,
    clothing_description: draft.profile.clothing_description,
    health_notes: draft.profile.health_notes,
    last_seen_location: draft.profile.last_seen_information ?? '',
  }
}

function syncProfileFields(current: ConfirmedIntakeProfile, draft: IntakeDraft, replacedFields: string[]) {
  const next = { ...current }
  for (const field of replacedFields) {
    switch (field) {
      case 'physical_description':
        next.physical_description = draft.profile.physical_description
        break
      case 'clothing_description':
        next.clothing_description = draft.profile.clothing_description
        break
      case 'health_notes':
        next.health_notes = draft.profile.health_notes
        break
      case 'last_seen_information':
        next.last_seen_location = draft.profile.last_seen_information ?? ''
        break
    }
  }
  return next
}

function normalizedProfile(profile: ConfirmedIntakeProfile): ConfirmedIntakeProfile {
  return {
    ...profile,
    display_name: profile.display_name.trim(),
    last_seen_location: profile.last_seen_location.trim(),
    gender: nullable(profile.gender ?? ''),
    physical_description: nullable(profile.physical_description ?? ''),
    clothing_description: nullable(profile.clothing_description ?? ''),
    health_notes: nullable(profile.health_notes ?? ''),
  }
}

function toStoredSession(session: IntakeSession): StoredIntakeSession {
  return {
    id: session.id,
    status: session.status,
    missing_fields: session.missing_fields,
    phase: session.phase,
    completed_phase_one_fields: session.completed_phase_one_fields,
    missing_phase_one_fields: session.missing_phase_one_fields,
    phase_transition_ready: session.phase_transition_ready,
    next_question: session.next_question,
    guidance_mode: session.guidance_mode,
    privacy_notice: session.privacy_notice,
  }
}

function readStoredState(storageKey: string): StoredIntakeState | null {
  try {
    const value = window.sessionStorage.getItem(storageKey)
    if (!value) return null
    const parsed = JSON.parse(value) as Partial<StoredIntakeState>
    const session = parsed.session
    if (!session || typeof session.id !== 'string') return discardStoredState(storageKey)
    if (!Array.isArray(session.missing_fields)) return discardStoredState(storageKey)
    if (!Array.isArray(session.completed_phase_one_fields)) return discardStoredState(storageKey)
    if (!Array.isArray(session.missing_phase_one_fields)) return discardStoredState(storageKey)
    if (!session.next_question && (session.status !== 'ready_for_confirmation')) return discardStoredState(storageKey)
    return {
      session: session as StoredIntakeSession,
      answer: typeof parsed.answer === 'string' ? parsed.answer : '',
    }
  } catch {
    return discardStoredState(storageKey)
  }
}

function discardStoredState(storageKey: string): null {
  window.sessionStorage.removeItem(storageKey)
  return null
}

function clearStoredState(storageKey: string) {
  if (typeof window !== 'undefined') window.sessionStorage.removeItem(storageKey)
}

function questionLabel(field: string) {
  return questionLabels[field] ?? field
}

function assessmentLabel(severity: IntakeAssessment['severity']) {
  return severity === 'blocking' ? '需要先处理' : severity === 'warning' ? '请注意' : '提示'
}

function assessmentClass(severity: IntakeAssessment['severity']) {
  return severity === 'blocking'
    ? 'border-red-200 bg-red-50 text-red-800'
    : severity === 'warning'
      ? 'border-amber-200 bg-amber-50 text-amber-900'
      : 'border-blue-200 bg-blue-50 text-blue-900'
}

function nullable(value: string): string | null {
  const trimmed = value.trim()
  return trimmed || null
}

function formatDate(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', { dateStyle: 'medium', timeStyle: 'short' }).format(date)
}

function toDateTimeLocal(value: string | null) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  const offset = date.getTimezoneOffset() * 60_000
  return new Date(date.getTime() - offset).toISOString().slice(0, 16)
}

function messageFrom(cause: unknown) {
  return cause instanceof ApiClientError
    ? cause.message
    : cause instanceof Error
      ? cause.message
      : '操作失败，请稍后重试。'
}
