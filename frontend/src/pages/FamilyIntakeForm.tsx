import { Button, Input, TextArea } from '@heroui/react'
import { useState } from 'react'
import { confirmIntakeSession, createIntakeSession, getIntakeDraft, submitIntakeAnswer } from '../api/intake'
import type { ConfirmedIntakeProfile, IntakeAssessment, IntakeDraft, IntakeSession } from '../api/intake'
import { ApiClientError } from '../api/client'
import { useAuth } from '../auth/useAuth'

const blankProfile: ConfirmedIntakeProfile = { display_name: '', age: null, gender: null, physical_description: null, clothing_description: null, health_notes: null, last_seen_at: null, last_seen_location: '' }

export function FamilyIntakeForm({ onCancel, onConfirmed }: { onCancel: () => void; onConfirmed: (caseId: string, caseCode: string) => Promise<void> }) {
  const { token } = useAuth()
  const [session, setSession] = useState<IntakeSession | null>(null)
  const [draft, setDraft] = useState<IntakeDraft | null>(null)
  const [answer, setAnswer] = useState('')
  const [profile, setProfile] = useState<ConfirmedIntakeProfile>(blankProfile)
  const [assessments, setAssessments] = useState<IntakeAssessment[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  async function begin() {
    if (!token) return
    setBusy(true); setError('')
    try { setSession(await createIntakeSession(token)) } catch (cause) { setError(messageFrom(cause)) } finally { setBusy(false) }
  }

  async function submitAnswer(value = answer) {
    if (!token || !session?.next_question || !value.trim()) { setError('请填写答案，或选择“标记为未知”。'); return }
    setBusy(true); setError('')
    try {
      const next = await submitIntakeAnswer(token, session.id, session.next_question.field, value.trim())
      setSession(next); setAssessments(next.assessments); setAnswer('')
      if (next.status === 'ready_for_confirmation') {
        const nextDraft = await getIntakeDraft(token, session.id)
        setDraft(nextDraft)
        setProfile({ ...blankProfile, physical_description: nextDraft.profile.physical_description, clothing_description: nextDraft.profile.clothing_description, health_notes: nextDraft.profile.health_notes })
      }
    } catch (cause) { setError(messageFrom(cause)) } finally { setBusy(false) }
  }

  async function confirm() {
    if (!token || !session || !draft) return
    if (draft.confirmation_blocked_reasons.length > 0) { setError('存在阻断性冲突，请修改、补充说明或将相关问题标记为未知后再确认。'); return }
    if (!profile.display_name.trim() || !profile.last_seen_location.trim()) { setError('请确认姓名/称呼和最后出现地点。'); return }
    setBusy(true); setError('')
    try {
      const response = await confirmIntakeSession(token, session.id, { ...profile, display_name: profile.display_name.trim(), last_seen_location: profile.last_seen_location.trim() })
      await onConfirmed(response.case_id, response.case_code)
    } catch (cause) { setError(messageFrom(cause)) } finally { setBusy(false) }
  }

  if (!session) return <section className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5"><h2 className="m-0 text-base font-bold text-slate-950">开始走失求助问询</h2><p className="mb-4 mt-2 text-sm leading-6 text-slate-600">分两步收集信息。所有内容都是待确认草稿；可随时补充或标记未知，系统不会把规则整理直接当作案件事实。</p>{error && <Alert>{error}</Alert>}<div className="flex gap-2"><Button variant="primary" onPress={() => void begin()} isDisabled={busy}>{busy ? '正在开始' : '开始问询'}</Button><Button variant="ghost" onPress={onCancel}>取消</Button></div></section>

  if (draft) return <section className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5"><h2 className="m-0 text-base font-bold text-slate-950">确认老人画像草稿</h2><p className="mt-2 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">AI/规则整理草稿，必须由家属人工确认；它不代表已核实事实。</p>{error && <Alert>{error}</Alert>}<AssessmentList items={draft.assessments} />{draft.direction_hypotheses.map((item, index) => <div key={index} className="mt-3 rounded-md border border-slate-200 p-3 text-sm"><strong>可能方向（不确定）</strong><p className="mb-1 mt-1">{item.description}</p><span className="text-xs text-slate-600">{item.uncertainty_notice}</span></div>)}<div className="mt-5 grid gap-3 sm:grid-cols-2"><Field label="姓名或称呼" required><Input value={profile.display_name} maxLength={120} onChange={(event) => setProfile({ ...profile, display_name: event.target.value })} fullWidth /></Field><Field label="年龄"><Input type="number" min={0} max={130} value={profile.age ?? ''} onChange={(event) => setProfile({ ...profile, age: event.target.value ? Number(event.target.value) : null })} fullWidth /></Field><Field label="性别"><Input value={profile.gender ?? ''} onChange={(event) => setProfile({ ...profile, gender: event.target.value || null })} fullWidth /></Field><Field label="最后出现地点" required><Input value={profile.last_seen_location} onChange={(event) => setProfile({ ...profile, last_seen_location: event.target.value })} fullWidth /></Field><Field label="最后出现时间"><Input type="datetime-local" onChange={(event) => setProfile({ ...profile, last_seen_at: event.target.value ? new Date(event.target.value).toISOString() : null })} fullWidth /></Field></div><div className="mt-3 grid gap-3"><Field label="体貌描述"><TextArea value={profile.physical_description ?? ''} onChange={(event) => setProfile({ ...profile, physical_description: event.target.value || null })} rows={2} fullWidth /></Field><Field label="衣着描述"><TextArea value={profile.clothing_description ?? ''} onChange={(event) => setProfile({ ...profile, clothing_description: event.target.value || null })} rows={2} fullWidth /></Field><Field label="健康注意事项"><TextArea value={profile.health_notes ?? ''} onChange={(event) => setProfile({ ...profile, health_notes: event.target.value || null })} rows={2} fullWidth /></Field></div><div className="mt-5 flex flex-wrap gap-2"><Button variant="primary" onPress={() => void confirm()} isDisabled={busy || draft.confirmation_blocked_reasons.length > 0}>{busy ? '正在创建案件' : '人工确认并创建案件'}</Button><Button variant="ghost" onPress={onCancel}>暂不确认</Button></div></section>

  const question = session.next_question
  return <section className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5"><div className="flex items-center justify-between gap-3"><div><h2 className="m-0 text-base font-bold text-slate-950">{session.phase === 'phase_one' ? '第一步：基本情况与最后出现' : '第二步：地点、动机、物品与后续线索'}</h2><p className="mb-0 mt-1 text-xs text-slate-500">缺失必填项：{session.missing_fields.length ? session.missing_fields.join('、') : '无'}；保存状态：{busy ? '保存中' : '已保存'}</p></div><span className="text-xs text-slate-500">规则问询</span></div>{error && <Alert>{error}</Alert>}<AssessmentList items={assessments} />{question ? <form className="mt-4" onSubmit={(event) => { event.preventDefault(); void submitAnswer() }}><Field label={question.prompt} required={question.required}><TextArea value={answer} onChange={(event) => setAnswer(event.target.value)} rows={4} maxLength={2000} fullWidth /></Field><div className="mt-3 flex flex-wrap gap-2"><Button type="submit" variant="primary" isDisabled={busy}>{busy ? '正在保存' : '保存并继续'}</Button><Button type="button" variant="ghost" onPress={() => void submitAnswer('未知')}>标记为未知</Button><Button type="button" variant="ghost" onPress={onCancel}>取消</Button></div></form> : <p className="mt-4 text-sm text-slate-600">正在整理草稿，请稍候。</p>}</section>
}

function AssessmentList({ items }: { items: IntakeAssessment[] }) { return items.length ? <div className="mt-4 space-y-2">{items.map((item, index) => <div key={`${item.field_path}-${index}`} className={`rounded-md border px-3 py-2 text-sm ${item.severity === 'blocking' ? 'border-red-200 bg-red-50 text-red-800' : item.severity === 'warning' ? 'border-amber-200 bg-amber-50 text-amber-900' : 'border-blue-200 bg-blue-50 text-blue-900'}`}><strong>{item.severity === 'blocking' ? '阻断性冲突' : item.severity === 'warning' ? '请注意' : '提示'}</strong><span className="ml-2">{item.evidence_summary}</span><p className="mb-0 mt-1 text-xs">{item.suggested_action}</p></div>)}</div> : null }
function Field({ label, required, children }: { label: string; required?: boolean; children: React.ReactNode }) { return <label className="block"><span className="mb-1.5 block text-xs font-semibold text-slate-600">{label}{required ? ' *' : ''}</span>{children}</label> }
function Alert({ children }: { children: React.ReactNode }) { return <div className="mt-3 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">{children}</div> }
function messageFrom(cause: unknown) { return cause instanceof ApiClientError ? cause.message : cause instanceof Error ? cause.message : '操作失败，请稍后重试。' }
