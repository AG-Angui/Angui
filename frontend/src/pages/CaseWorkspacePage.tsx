import { Button, Chip, Input, TextArea } from '@heroui/react'
import {
  CheckCircle2,
  ChevronRight,
  CirclePlus,
  FileSearch,
  MapPin,
  RefreshCw,
  Send,
  UserPlus,
} from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  addCaseMember,
  createCase,
  createClue,
  getCase,
  listCases,
  reviewClue,
  updateCaseStatus,
} from '../api/cases'
import type {
  CaseDetail,
  CaseListItem,
  CaseRole,
  CaseStatus,
  ClueReviewStatus,
  CreateCasePayload,
} from '../api/cases'
import { ApiClientError } from '../api/client'
import { useAuth } from '../auth/useAuth'
import { EmptyState, ErrorState, LoadingState } from '../components/ContentState'

type WorkspaceMode = 'family' | 'commander' | 'volunteer'

const workspaceCopy: Record<WorkspaceMode, { context: string; title: string }> = {
  family: { context: '家属端', title: '走失求助' },
  commander: { context: '指挥端', title: '案件指挥' },
  volunteer: { context: '志愿者端', title: '协作案件' },
}

const statusLabels: Record<string, string> = {
  active: '进行中',
  resolved: '已找到',
  closed: '已关闭',
  pending_review: '待审核',
  needs_verification: '待核实',
  confirmed: '已确认',
  rejected: '已排除',
  expired: '已失效',
  duplicate: '重复',
}

const emptyCase: CreateCasePayload = {
  display_name: '',
  age: null,
  gender: null,
  physical_description: null,
  clothing_description: null,
  health_notes: null,
  last_seen_at: null,
  last_seen_location: '',
}

export function CaseWorkspacePage({ mode }: { mode: WorkspaceMode }) {
  const { token } = useAuth()
  const [cases, setCases] = useState<CaseListItem[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [detail, setDetail] = useState<CaseDetail | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isDetailLoading, setIsDetailLoading] = useState(false)
  const [listError, setListError] = useState('')
  const [detailError, setDetailError] = useState('')
  const [notice, setNotice] = useState('')
  const [showCreate, setShowCreate] = useState(false)

  const loadCases = useCallback(async (preferredId?: string) => {
    if (!token) return
    setIsLoading(true)
    setListError('')
    try {
      const items = await listCases(token)
      setCases(items)
      setSelectedId((currentId) => {
        const nextId = preferredId ?? currentId ?? items[0]?.id ?? null
        if (!nextId) setDetail(null)
        return nextId
      })
    } catch (cause) {
      setListError(messageFrom(cause))
    } finally {
      setIsLoading(false)
    }
  }, [token])

  const loadDetail = useCallback(async (caseId: string) => {
    if (!token) return
    setIsDetailLoading(true)
    setDetailError('')
    try {
      setDetail(await getCase(token, caseId))
    } catch (cause) {
      setDetailError(messageFrom(cause))
      setDetail(null)
    } finally {
      setIsDetailLoading(false)
    }
  }, [token])

  useEffect(() => {
    void loadCases()
  }, [loadCases])

  useEffect(() => {
    if (selectedId) void loadDetail(selectedId)
  }, [loadDetail, selectedId])

  const pendingCount = useMemo(
    () => detail?.clues.filter((clue) => clue.status === 'pending_review').length ?? 0,
    [detail],
  )
  const copy = workspaceCopy[mode]

  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-6 flex min-h-14 flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <span className="mb-1 block text-xs font-semibold text-slate-500">{copy.context}</span>
          <h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">{copy.title}</h1>
        </div>
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onPress={() => void loadCases()}>
            <RefreshCw size={16} />
            刷新
          </Button>
          {mode !== 'volunteer' && (
            <Button size="sm" variant="primary" onPress={() => setShowCreate((value) => !value)}>
              <CirclePlus size={16} />
              新建案件
            </Button>
          )}
        </div>
      </header>

      {listError && cases.length > 0 && <Message tone="error">{listError}</Message>}
      {notice && <Message tone="success">{notice}</Message>}

      {showCreate && mode !== 'volunteer' && (
        <CreateCaseForm
          onCancel={() => setShowCreate(false)}
          onCreated={async (created) => {
            setShowCreate(false)
            setNotice(`案件 ${created.case_code} 已建立`)
            await loadCases(created.id)
          }}
        />
      )}

      <div className="mt-5 grid min-h-[560px] overflow-hidden border-y border-slate-200 bg-white lg:grid-cols-[310px_minmax(0,1fr)]">
        <section className="border-b border-slate-200 lg:border-r lg:border-b-0" aria-label="案件列表">
          <div className="flex min-h-16 items-center justify-between border-b border-slate-200 px-4 py-3">
            <strong className="text-sm text-slate-950">可访问案件</strong>
            <Chip size="sm" variant="soft"><Chip.Label>{cases.length}</Chip.Label></Chip>
          </div>
          {isLoading ? (
            <LoadingState label="正在加载可访问案件" />
          ) : listError && cases.length === 0 ? (
            <ErrorState message={listError} onRetry={() => void loadCases()} />
          ) : cases.length === 0 ? (
            <EmptyState title="暂无案件" description="新建案件后，会显示在这里。" />
          ) : (
            <div className="divide-y divide-slate-100">
              {cases.map((item) => (
                <button
                  type="button"
                  key={item.id}
                  onClick={() => setSelectedId(item.id)}
                  aria-pressed={selectedId === item.id}
                  className={`flex min-h-20 w-full items-center gap-3 px-4 py-3 text-left transition-colors ${
                    selectedId === item.id ? 'bg-brand-50' : 'hover:bg-slate-50'
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <strong className="truncate text-sm text-slate-950">{item.display_name}</strong>
                      <Chip size="sm" variant="soft"><Chip.Label>{statusLabels[item.status]}</Chip.Label></Chip>
                    </div>
                    <span className="mt-1 block truncate text-xs text-slate-500">{item.case_code}</span>
                    <span className="mt-0.5 block truncate text-xs text-slate-500">{item.last_seen_location ?? '地点待补充'}</span>
                  </div>
                  <ChevronRight size={17} className="shrink-0 text-slate-400" />
                </button>
              ))}
            </div>
          )}
        </section>

        <section className="min-w-0">
          {isDetailLoading ? (
            <LoadingState label="正在加载案件详情" />
          ) : detailError ? (
            <ErrorState
              message={detailError}
              onRetry={() => selectedId && void loadDetail(selectedId)}
            />
          ) : detail ? (
            <CaseDetailView
              detail={detail}
              pendingCount={pendingCount}
              onChanged={async (message) => {
                setNotice(message)
                await loadDetail(detail.id)
                await loadCases(detail.id)
              }}
            />
          ) : (
            <div className="flex min-h-96 flex-col items-center justify-center px-6 text-center">
              <EmptyState icon={FileSearch} title="选择一个案件查看详情" />
            </div>
          )}
        </section>
      </div>
    </div>
  )
}

function CreateCaseForm({
  onCancel,
  onCreated,
}: {
  onCancel: () => void
  onCreated: (detail: CaseDetail) => Promise<void>
}) {
  const { token } = useAuth()
  const [form, setForm] = useState<CreateCasePayload>(emptyCase)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!token) return
    const validationError = validateCreateCase(form)
    if (validationError) {
      setError(validationError)
      return
    }
    setIsSubmitting(true)
    setError('')
    try {
      await onCreated(
        await createCase(token, {
          ...form,
          display_name: form.display_name.trim(),
          last_seen_location: form.last_seen_location?.trim() ?? '',
          last_seen_at: form.last_seen_at ? new Date(form.last_seen_at).toISOString() : null,
        }),
      )
      setForm(emptyCase)
    } catch (cause) {
      setError(messageFrom(cause))
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <section className="border-y border-slate-200 bg-white px-4 py-5 sm:px-5" aria-labelledby="create-case-title">
      <h2 id="create-case-title" className="m-0 text-base font-bold text-slate-950">建立模拟案件</h2>
      <form onSubmit={submit} className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <Field label="案件内称呼" required>
          <Input value={form.display_name} maxLength={120} onChange={(event) => setForm({ ...form, display_name: event.target.value })} fullWidth required />
        </Field>
        <Field label="年龄">
          <Input type="number" min={0} max={130} step={1} value={form.age ?? ''} onChange={(event) => setForm({ ...form, age: event.target.value ? Number(event.target.value) : null })} fullWidth />
        </Field>
        <Field label="性别">
          <Input value={form.gender ?? ''} onChange={(event) => setForm({ ...form, gender: nullable(event.target.value) })} fullWidth />
        </Field>
        <Field label="最后出现地点" required>
          <Input value={form.last_seen_location ?? ''} onChange={(event) => setForm({ ...form, last_seen_location: event.target.value })} fullWidth required />
        </Field>
        <Field label="最后出现时间">
          <Input type="datetime-local" value={form.last_seen_at ?? ''} onChange={(event) => setForm({ ...form, last_seen_at: nullable(event.target.value) })} fullWidth />
        </Field>
        <Field label="衣着描述">
          <Input value={form.clothing_description ?? ''} onChange={(event) => setForm({ ...form, clothing_description: nullable(event.target.value) })} fullWidth />
        </Field>
        <div className="sm:col-span-2 lg:col-span-3">
          <Field label="体貌描述"><TextArea value={form.physical_description ?? ''} onChange={(event) => setForm({ ...form, physical_description: nullable(event.target.value) })} fullWidth rows={3} /></Field>
        </div>
        <div className="sm:col-span-2 lg:col-span-3">
          <Field label="健康注意事项"><TextArea value={form.health_notes ?? ''} onChange={(event) => setForm({ ...form, health_notes: nullable(event.target.value) })} fullWidth rows={3} /></Field>
        </div>
        {error && <div className="sm:col-span-2 lg:col-span-3"><Message tone="error">{error}</Message></div>}
        <div className="flex gap-2 sm:col-span-2 lg:col-span-3">
          <Button type="submit" variant="primary" isDisabled={isSubmitting}><CirclePlus size={16} />{isSubmitting ? '正在建立' : '建立案件'}</Button>
          <Button type="button" variant="ghost" onPress={onCancel}>取消</Button>
        </div>
      </form>
    </section>
  )
}

function CaseDetailView({
  detail,
  pendingCount,
  onChanged,
}: {
  detail: CaseDetail
  pendingCount: number
  onChanged: (message: string) => Promise<void>
}) {
  const { token } = useAuth()
  const [clueContent, setClueContent] = useState('')
  const [clueLocation, setClueLocation] = useState('')
  const [clueOccurredAt, setClueOccurredAt] = useState('')
  const [nextStatus, setNextStatus] = useState<CaseStatus>(detail.status)
  const [memberEmail, setMemberEmail] = useState('')
  const [memberRole, setMemberRole] = useState<CaseRole>('volunteer')
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')
  const isCommander = detail.access_role === 'commander'
  const statusOptions: CaseStatus[] = detail.status === 'active'
    ? ['active', 'resolved', 'closed']
    : detail.status === 'resolved'
      ? ['resolved', 'active', 'closed']
      : ['closed']

  useEffect(() => setNextStatus(detail.status), [detail.status])

  async function run(key: string, action: () => Promise<unknown>, message: string) {
    setBusy(key)
    setError('')
    try {
      await action()
      await onChanged(message)
      return true
    } catch (cause) {
      setError(messageFrom(cause))
      return false
    } finally {
      setBusy('')
    }
  }

  return (
    <div>
      <header className="border-b border-slate-200 px-5 py-4 sm:px-6">
        <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="m-0 text-xl font-bold text-slate-950">{detail.elder_profile.display_name}</h2>
              <Chip size="sm" variant="soft"><Chip.Label>{statusLabels[detail.status]}</Chip.Label></Chip>
              {pendingCount > 0 && <Chip size="sm" variant="soft"><Chip.Label>{pendingCount} 条待审核</Chip.Label></Chip>}
            </div>
            <span className="mt-1 block text-xs text-slate-500">{detail.case_code}</span>
          </div>
          <span className="text-xs text-slate-500">权限：{detail.access_role}</span>
        </div>
      </header>

      <section className="grid gap-x-8 gap-y-4 border-b border-slate-200 px-5 py-5 sm:grid-cols-2 sm:px-6 lg:grid-cols-3" aria-label="老人资料">
        <Info label="最后出现地点" value={detail.elder_profile.last_seen_location} icon={<MapPin size={16} />} />
        <Info label="最后出现时间" value={formatDate(detail.elder_profile.last_seen_at)} />
        <Info label="年龄" value={detail.elder_profile.age == null ? null : `${detail.elder_profile.age} 岁`} />
        <Info label="体貌" value={detail.elder_profile.physical_description} />
        <Info label="衣着" value={detail.elder_profile.clothing_description} />
        {detail.elder_profile.health_notes && <Info label="健康注意" value={detail.elder_profile.health_notes} />}
      </section>

      {(isCommander || detail.access_role === 'family') && (
        <section className="grid gap-5 border-b border-slate-200 bg-slate-50 px-5 py-5 sm:px-6 lg:grid-cols-2">
          {isCommander && <form
            onSubmit={(event) => {
              event.preventDefault()
              if (!token) return
              void run('status', () => updateCaseStatus(token, detail.id, nextStatus), '案件状态已更新')
            }}
          >
            <h3 className="m-0 text-sm font-bold text-slate-950">案件状态</h3>
            <div className="mt-3 flex gap-2">
              <select className="min-h-10 flex-1 rounded-md border border-slate-300 bg-white px-3 text-sm" value={nextStatus} onChange={(event) => setNextStatus(event.target.value as CaseStatus)} disabled={detail.status === 'closed'}>
                {statusOptions.map((status) => <option key={status} value={status}>{statusLabels[status]}</option>)}
              </select>
              <Button type="submit" size="sm" variant="secondary" isDisabled={busy === 'status' || nextStatus === detail.status}>保存</Button>
            </div>
          </form>}

          <form
            onSubmit={(event) => {
              event.preventDefault()
              if (!token) return
              const role = detail.access_role === 'family' ? 'commander' : memberRole
              void run('member', () => addCaseMember(token, detail.id, memberEmail.trim(), role), '案件成员已添加').then((succeeded) => {
                if (succeeded) setMemberEmail('')
              })
            }}
          >
            <h3 className="m-0 text-sm font-bold text-slate-950">添加成员</h3>
            <div className="mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_120px_auto]">
              <Input type="email" value={memberEmail} maxLength={320} onChange={(event) => setMemberEmail(event.target.value)} placeholder="成员邮箱" fullWidth required />
              <select className="min-h-10 rounded-md border border-slate-300 bg-white px-3 text-sm" value={detail.access_role === 'family' ? 'commander' : memberRole} onChange={(event) => setMemberRole(event.target.value as CaseRole)} disabled={detail.access_role === 'family'}>
                {detail.access_role === 'family' ? (
                  <option value="commander">指挥</option>
                ) : (
                  <>
                    <option value="family">家属</option>
                    <option value="commander">指挥</option>
                    <option value="volunteer">志愿者</option>
                  </>
                )}
              </select>
              <Button type="submit" size="sm" variant="secondary" isDisabled={busy === 'member'} isIconOnly aria-label="添加案件成员"><UserPlus size={17} /></Button>
            </div>
          </form>
        </section>
      )}

      {error && <div className="px-5 pt-4 sm:px-6"><Message tone="error">{error}</Message></div>}

      <section className="px-5 py-5 sm:px-6" aria-labelledby="clues-title">
        <div className="flex items-center justify-between gap-3">
          <h3 id="clues-title" className="m-0 text-base font-bold text-slate-950">线索</h3>
          <Chip size="sm" variant="soft"><Chip.Label>{detail.clues.length} 条可见</Chip.Label></Chip>
        </div>

        <div className="mt-4 divide-y divide-slate-100 border-y border-slate-200">
          {detail.clues.length === 0 ? (
            <div className="flex min-h-28 items-center justify-center text-sm text-slate-500">暂无可见线索</div>
          ) : detail.clues.map((clue) => (
            <article key={clue.id} className="py-4">
              <div className="flex flex-wrap items-center gap-2">
                <Chip size="sm" variant="soft"><Chip.Label>{statusLabels[clue.status] ?? clue.status}</Chip.Label></Chip>
                <span className="text-xs text-slate-500">{clue.source}</span>
                {clue.is_own_submission && <span className="text-xs font-medium text-brand-700">本人提交</span>}
                <span className="ml-auto text-xs text-slate-500">{formatDate(clue.occurred_at ?? clue.created_at)}</span>
              </div>
              <p className="m-0 mt-2 whitespace-pre-wrap text-sm leading-6 text-slate-700">{clue.content}</p>
              {clue.location_text && <p className="m-0 mt-1 text-xs text-slate-500">{clue.location_text}</p>}
              {isCommander && clue.status === 'pending_review' && (
                <div className="mt-3 flex flex-wrap gap-2">
                  <ReviewButton label="确认" status="confirmed" clueId={clue.id} busy={busy} run={run} />
                  <ReviewButton label="待核实" status="needs_verification" clueId={clue.id} busy={busy} run={run} />
                  <ReviewButton label="排除" status="rejected" clueId={clue.id} busy={busy} run={run} />
                  <ReviewButton label="重复" status="duplicate" clueId={clue.id} busy={busy} run={run} />
                </div>
              )}
            </article>
          ))}
        </div>

        {detail.status !== 'closed' && (
          <form
            className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,1fr)_220px]"
            onSubmit={(event) => {
              event.preventDefault()
              if (!token) return
              const content = nullable(clueContent)
              if (!content) {
                setError('请填写线索内容后再提交。')
                return
              }
              void run(
                'clue',
                () => createClue(token, detail.id, { source: detail.access_role, content, occurred_at: toIsoOrNull(clueOccurredAt), location_text: nullable(clueLocation) }),
                '线索已提交并进入人工审核',
              ).then((succeeded) => {
                if (succeeded) {
                  setClueContent('')
                  setClueLocation('')
                  setClueOccurredAt('')
                }
              })
            }}
          >
            <Field label="线索内容" required><TextArea value={clueContent} maxLength={4000} onChange={(event) => setClueContent(event.target.value)} rows={3} fullWidth required /></Field>
            <div className="space-y-3">
              <Field label="发生时间"><Input type="datetime-local" value={clueOccurredAt} onChange={(event) => setClueOccurredAt(event.target.value)} fullWidth /></Field>
              <Field label="地点"><Input value={clueLocation} onChange={(event) => setClueLocation(event.target.value)} fullWidth /></Field>
              <Button type="submit" variant="primary" fullWidth isDisabled={busy === 'clue'}><Send size={16} />提交线索</Button>
            </div>
          </form>
        )}
      </section>
    </div>
  )
}

function ReviewButton({
  label,
  status,
  clueId,
  busy,
  run,
}: {
  label: string
  status: ClueReviewStatus
  clueId: string
  busy: string
  run: (key: string, action: () => Promise<unknown>, message: string) => Promise<boolean>
}) {
  const { token } = useAuth()
  const key = `review:${clueId}:${status}`
  return (
    <Button
      size="sm"
      variant={status === 'confirmed' ? 'secondary' : 'ghost'}
      isDisabled={busy === key}
      onPress={() => token && void run(key, () => reviewClue(token, clueId, status), `线索已更新为${statusLabels[status]}`)}
    >
      {status === 'confirmed' && <CheckCircle2 size={15} />}
      {label}
    </Button>
  )
}

function Field({ label, required, children }: { label: string; required?: boolean; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-xs font-semibold text-slate-600">{label}{required ? ' *' : ''}</span>
      {children}
    </label>
  )
}

function Info({ label, value, icon }: { label: string; value: string | null; icon?: React.ReactNode }) {
  return (
    <div className="min-w-0">
      <span className="flex items-center gap-1 text-xs text-slate-500">{icon}{label}</span>
      <strong className="mt-1 block whitespace-pre-wrap text-sm font-medium text-slate-800">{value || '未填写'}</strong>
    </div>
  )
}

function Message({ tone, children }: { tone: 'error' | 'success'; children: React.ReactNode }) {
  return (
    <div className={`mb-3 rounded-md border px-3 py-2 text-sm ${tone === 'error' ? 'border-red-200 bg-red-50 text-red-700' : 'border-emerald-200 bg-emerald-50 text-emerald-700'}`} role={tone === 'error' ? 'alert' : 'status'}>
      {children}
    </div>
  )
}

function nullable(value: string): string | null {
  const trimmed = value.trim()
  return trimmed ? trimmed : null
}

function validateCreateCase(form: CreateCasePayload): string | null {
  if (!form.display_name.trim()) return '请填写案件内称呼。'
  if (!form.last_seen_location?.trim()) return '请填写最后出现地点。'
  if (form.age !== null && (!Number.isInteger(form.age) || form.age < 0 || form.age > 130)) {
    return '年龄应为 0 到 130 之间的整数。'
  }
  if (form.last_seen_at && Number.isNaN(new Date(form.last_seen_at).getTime())) {
    return '最后出现时间格式无效。'
  }
  return null
}

function toIsoOrNull(value: string): string | null {
  if (!value) return null
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? null : date.toISOString()
}

function messageFrom(cause: unknown): string {
  if (cause instanceof ApiClientError) return cause.message
  return cause instanceof Error ? cause.message : '操作失败'
}

function formatDate(value: string | null): string | null {
  if (!value) return null
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false })
}
