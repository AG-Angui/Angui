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
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  addCaseMember,
  createCasePlace,
  createClue,
  getCase,
  getCaseResourceConfiguration,
  listCases,
  reviewClue,
  updateCaseStatus,
  uploadCaseAttachment,
} from '../api/cases'
import type {
  CaseDetail,
  CaseListItem,
  CaseRole,
  CaseResourceConfiguration,
  CaseStatus,
  ClueReviewStatus,
  CreateCasePlacePayload,
  PlaceType,
  PlaceVisibility,
} from '../api/cases'
import { ApiClientError } from '../api/client'
import { useAuth } from '../auth/useAuth'
import { EmptyState, ErrorState, LoadingState } from '../components/ContentState'
import { FamilyIntakeForm } from './FamilyIntakeForm'

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

const placeTypeLabels: Record<string, string> = {
  frequent: '常去地点',
  key_location: '关键地点',
  last_seen_context: '最后出现相关',
  medical: '医疗',
  shelter: '临时安置',
  other: '其他',
}

export function CaseWorkspacePage({ mode }: { mode: WorkspaceMode }) {
  const { token, user } = useAuth()
  const [cases, setCases] = useState<CaseListItem[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [detail, setDetail] = useState<CaseDetail | null>(null)
  const [resourceConfiguration, setResourceConfiguration] = useState<CaseResourceConfiguration | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isDetailLoading, setIsDetailLoading] = useState(false)
  const [listError, setListError] = useState('')
  const [detailError, setDetailError] = useState('')
  const [notice, setNotice] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const detailRequestVersion = useRef(0)

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
    const requestVersion = detailRequestVersion.current + 1
    detailRequestVersion.current = requestVersion
    if (!token) return
    setIsDetailLoading(true)
    setDetailError('')
    try {
      const [nextDetail, nextResourceConfiguration] = await Promise.all([
        getCase(token, caseId),
        getCaseResourceConfiguration(token, caseId),
      ])
      if (requestVersion !== detailRequestVersion.current) return
      setDetail(nextDetail)
      setResourceConfiguration(nextResourceConfiguration)
    } catch (cause) {
      if (requestVersion !== detailRequestVersion.current) return
      setDetailError(messageFrom(cause))
      setDetail(null)
      setResourceConfiguration(null)
    } finally {
      if (requestVersion === detailRequestVersion.current) setIsDetailLoading(false)
    }
  }, [token])

  useEffect(() => {
    void loadCases()
  }, [loadCases])

  useEffect(() => {
    if (selectedId) {
      void loadDetail(selectedId)
      return
    }
    detailRequestVersion.current += 1
    setDetail(null)
    setResourceConfiguration(null)
    setDetailError('')
    setIsDetailLoading(false)
  }, [loadDetail, selectedId])

  const pendingCount = useMemo(
    () => detail?.clues.filter((clue) => clue.status === 'pending_review').length ?? 0,
    [detail],
  )
  const copy = workspaceCopy[mode]
  const canCreateCase = user?.account_type === 'member'

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
          {canCreateCase && mode !== 'volunteer' && (
            <Button size="sm" variant="primary" onPress={() => setShowCreate((value) => !value)}>
              <CirclePlus size={16} />
              新建案件
            </Button>
          )}
        </div>
      </header>

      {listError && cases.length > 0 && <Message tone="error">{listError}</Message>}
      {notice && <Message tone="success">{notice}</Message>}

      {showCreate && canCreateCase && mode !== 'volunteer' && (
        <FamilyIntakeForm
          onCancel={() => setShowCreate(false)}
          onConfirmed={async (caseId, caseCode) => {
            setShowCreate(false)
            setNotice(`案件 ${caseCode} 已由家属人工确认创建`)
            await loadCases(caseId)
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
          ) : detail && resourceConfiguration ? (
            <CaseDetailView
              detail={detail}
              resourceConfiguration={resourceConfiguration}
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

function CaseDetailView({
  detail,
  resourceConfiguration,
  pendingCount,
  onChanged,
}: {
  detail: CaseDetail
  resourceConfiguration: CaseResourceConfiguration
  pendingCount: number
  onChanged: (message: string) => Promise<void>
}) {
  const { token } = useAuth()
  const [clueContent, setClueContent] = useState('')
  const [clueLocation, setClueLocation] = useState('')
  const [clueOccurredAt, setClueOccurredAt] = useState('')
  const [place, setPlace] = useState<CreateCasePlacePayload>({ name: '', place_type: '', address: '', longitude: null, latitude: null, visibility: 'confirmed' })
  const [attachment, setAttachment] = useState<File | null>(null)
  const [nextStatus, setNextStatus] = useState<CaseStatus>(detail.status)
  const [memberEmail, setMemberEmail] = useState('')
  const [memberRole, setMemberRole] = useState<CaseRole>('volunteer')
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')
  const isCommander = detail.access_role === 'commander'
  const canSubmitPlace = detail.access_role === 'family' || isCommander
  const placeTypes = resourceConfiguration.case_place_types
  const statusOptions: CaseStatus[] = detail.status === 'active'
    ? ['active', 'resolved', 'closed']
    : detail.status === 'resolved'
      ? ['resolved', 'active', 'closed']
      : ['closed']

  useEffect(() => setNextStatus(detail.status), [detail.status])
  useEffect(() => {
    setPlace((current) => (
      placeTypes.includes(current.place_type)
        ? current
        : { ...current, place_type: placeTypes[0] ?? '' }
    ))
  }, [placeTypes])

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
            <h3 id="case-status-title" className="m-0 text-sm font-bold text-slate-950">案件状态</h3>
            <div className="mt-3 flex gap-2">
              <select aria-labelledby="case-status-title" className="min-h-10 flex-1 rounded-md border border-slate-300 bg-white px-3 text-sm" value={nextStatus} onChange={(event) => setNextStatus(event.target.value as CaseStatus)} disabled={detail.status === 'closed'}>
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

      <section className={`grid gap-6 border-t border-slate-200 bg-slate-50 px-5 py-5 sm:px-6 ${canSubmitPlace ? 'lg:grid-cols-2' : ''}`} aria-label="补充地点和图片">
          {canSubmitPlace && <div>
            <div className="flex items-center justify-between gap-3">
              <h3 className="m-0 text-base font-bold text-slate-950">补充地点</h3>
              <span className="text-xs text-slate-500">提交后待人工审核</span>
            </div>
            <div className="mt-3 divide-y divide-slate-200 rounded-md border border-slate-200 bg-white">
              {detail.places.length === 0 ? <p className="m-0 px-3 py-3 text-xs text-slate-500">暂无可查看的补充地点</p> : detail.places.map((item) => (
                <div key={item.id} className="px-3 py-3 text-sm">
                  <strong className="text-slate-900">{item.name}</strong><span className="ml-2 text-xs text-slate-500">{item.review_status === 'pending_review' ? '待人工审核' : item.review_status}</span>
                  <p className="m-0 mt-1 text-xs text-slate-600">{item.address}</p>
                </div>
              ))}
            </div>
            {detail.status !== 'closed' && <form className="mt-3 grid gap-3" onSubmit={(event) => {
              event.preventDefault()
              if (!token || !place.name.trim() || !place.address.trim() || !place.place_type) { setError('请填写地点名称、类型和文字地址后再提交。'); return }
              if ((place.longitude === null) !== (place.latitude === null)) { setError('经度和纬度必须同时填写或同时留空。'); return }
              void run('place', () => createCasePlace(token, detail.id, { ...place, name: place.name.trim(), address: place.address.trim() }), '地点已提交，正在等待人工审核').then((ok) => {
                if (ok) setPlace({ name: '', place_type: placeTypes[0] ?? '', address: '', longitude: null, latitude: null, visibility: 'confirmed' })
              })
            }}>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label="地点名称" required><Input value={place.name} maxLength={120} onChange={(event) => setPlace({ ...place, name: event.target.value })} fullWidth required /></Field>
                <Field label="类型"><select className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm" value={place.place_type} onChange={(event) => setPlace({ ...place, place_type: event.target.value as PlaceType })}>{placeTypes.map((type) => <option key={type} value={type}>{placeTypeLabels[type] ?? type}</option>)}</select></Field>
              </div>
              <Field label="文字地址" required><Input value={place.address} maxLength={500} onChange={(event) => setPlace({ ...place, address: event.target.value })} fullWidth required /></Field>
              <div className="grid gap-3 sm:grid-cols-3">
                <Field label="经度（可选）"><Input type="number" min={-180} max={180} value={place.longitude ?? ''} onChange={(event) => setPlace({ ...place, longitude: event.target.value === '' ? null : Number(event.target.value) })} fullWidth /></Field>
                <Field label="纬度（可选）"><Input type="number" min={-90} max={90} value={place.latitude ?? ''} onChange={(event) => setPlace({ ...place, latitude: event.target.value === '' ? null : Number(event.target.value) })} fullWidth /></Field>
                <Field label="可见级别"><select className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm" value={place.visibility} onChange={(event) => setPlace({ ...place, visibility: event.target.value as PlaceVisibility })}><option value="confirmed">已确认范围</option><option value="internal">仅内部</option><option value="public">公开范围</option></select></Field>
              </div>
              <Button type="submit" variant="secondary" isDisabled={busy === 'place'}>提交地点</Button>
            </form>}
          </div>}
          <div>
            <div className="flex items-center justify-between gap-3"><h3 className="m-0 text-base font-bold text-slate-950">补充图片</h3><span className="text-xs text-slate-500">仅 JPEG/PNG，最大 {formatBytes(resourceConfiguration.attachment_max_image_bytes)}</span></div>
            <div className="mt-3 divide-y divide-slate-200 rounded-md border border-slate-200 bg-white">
              {detail.attachments.length === 0 ? <p className="m-0 px-3 py-3 text-xs text-slate-500">暂无可查看的图片</p> : detail.attachments.map((item) => <div key={item.id} className="flex items-center justify-between gap-2 px-3 py-3 text-sm"><span className="truncate text-slate-800">{item.original_filename}</span><span className="shrink-0 text-xs text-slate-500">{item.review_status === 'pending_review' ? '待人工审核' : item.review_status}</span></div>)}
            </div>
            {detail.status !== 'closed' && <form className="mt-3 grid gap-3" onSubmit={(event) => {
              event.preventDefault()
              if (!token || !attachment) { setError('请选择一张图片后再提交。'); return }
              void run('attachment', () => uploadCaseAttachment(token, detail.id, attachment, resourceConfiguration.attachment_max_image_bytes), '图片已提交，正在等待人工审核').then((ok) => { if (ok) setAttachment(null) })
            }}>
              <input key={attachment ? `${attachment.name}-${attachment.lastModified}` : 'no-file'} type="file" accept="image/jpeg,image/png" onChange={(event) => setAttachment(event.target.files?.[0] ?? null)} className="block w-full text-sm text-slate-700" />
              <Button type="submit" variant="secondary" isDisabled={busy === 'attachment'}>上传图片</Button>
              <p className="m-0 text-xs leading-5 text-slate-500">上传会由服务端重新编码并移除非必要的 EXIF/GPS 元数据；失败时不会显示为上传成功。</p>
            </form>}
          </div>
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

function formatBytes(value: number): string {
  return value >= 1024 * 1024 ? `${(value / (1024 * 1024)).toFixed(1)} MiB` : `${value} 字节`
}
