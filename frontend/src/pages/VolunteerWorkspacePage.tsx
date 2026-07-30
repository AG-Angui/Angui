import { Button, Chip, Input, TextArea } from '@heroui/react'
import { AlertTriangle, Compass, LocateFixed, RefreshCw, ShieldCheck } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import {
  getTaskNavigation,
  getTaskSafetyBriefing,
  listMyTasks,
  submitTaskFeedback,
  submitTaskLocationReport,
  updateTaskStatus,
} from '../api/cases'
import type { CaseTask, TaskNavigation, TaskSafetyBriefing, TaskStatus } from '../api/cases'
import { useAuth } from '../auth/useAuth'
import { EmptyState, ErrorState, LoadingState } from '../components/ContentState'

const statusLabels: Record<TaskStatus, string> = {
  assigned: '待接取', accepted: '已接取', active: '执行中', blocked: '已暂停', completed: '已完成', cancelled: '已取消',
}

function messageFrom(cause: unknown) { return cause instanceof Error ? cause.message : '操作暂时无法完成，请稍后重试。' }
function localNow() { return new Date().toISOString() }

export function VolunteerWorkspacePage() {
  const { token } = useAuth()
  const [tasks, setTasks] = useState<CaseTask[]>([])
  const [navigation, setNavigation] = useState<Record<string, TaskNavigation>>({})
  const [safety, setSafety] = useState<Record<string, TaskSafetyBriefing>>({})
  const [feedback, setFeedback] = useState<Record<string, string>>({})
  const [location, setLocation] = useState<Record<string, { latitude: string; longitude: string; accuracy: string }>>({})
  const [busy, setBusy] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const load = useCallback(async () => {
    if (!token) return
    setLoading(true); setError('')
    try { setTasks(await listMyTasks(token)) } catch (cause) { setError(messageFrom(cause)) } finally { setLoading(false) }
  }, [token])
  useEffect(() => { void load() }, [load])

  async function run(key: string, action: () => Promise<void>, success: string) {
    setBusy(key); setError(''); setNotice('')
    try { await action(); setNotice(success) } catch (cause) { setError(messageFrom(cause)) } finally { setBusy('') }
  }
  async function changeStatus(task: CaseTask, status: TaskStatus) {
    if (!token) return
    await run(`status-${task.id}`, async () => {
      const updated = await updateTaskStatus(token, task.id, status)
      setTasks((current) => current.map((item) => item.id === task.id ? updated : item))
    }, `任务已更新为“${statusLabels[status]}”。`)
  }
  function statusActions(status: TaskStatus): TaskStatus[] {
    if (status === 'assigned') return ['accepted']
    if (status === 'accepted') return ['active']
    if (status === 'active') return ['blocked', 'completed']
    if (status === 'blocked') return ['active']
    return []
  }

  return <main className="mx-auto w-full max-w-5xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
    <header className="mb-7 flex flex-col items-start justify-between gap-3 sm:flex-row sm:items-end"><div><span className="mb-1 block text-xs font-semibold text-slate-500">志愿者执行</span><h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">我的任务</h1></div><Button size="sm" variant="ghost" isDisabled={loading} onPress={() => void load()}><RefreshCw size={16} />刷新</Button></header>
    {notice && <p className="mb-4 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">{notice}</p>}
    {error && !loading && <div className="mb-4"><ErrorState message={error} onRetry={() => void load()} /></div>}
    {loading ? <LoadingState label="正在加载分配给你的任务" /> : tasks.length === 0 ? <EmptyState icon={Compass} title="当前没有分配给你的任务" description="任务被指挥分配并授权后，会显示在这里。" /> : <section className="grid gap-4" aria-label="我的任务列表">{tasks.map((task) => {
      const active = task.status === 'active'
      const taskLocation = location[task.id] ?? { latitude: '', longitude: '', accuracy: '50' }
      return <article key={task.id} className="rounded-md border border-slate-200 bg-white p-4 shadow-sm"><div className="flex flex-wrap items-start justify-between gap-3"><div><h2 className="m-0 text-base font-bold text-slate-950">{task.title}</h2><p className="mb-0 mt-1 text-sm text-slate-700">{task.objective}</p></div><Chip size="sm" variant="soft"><Chip.Label>{statusLabels[task.status]}</Chip.Label></Chip></div>
        <dl className="mt-3 grid gap-2 text-sm text-slate-600 sm:grid-cols-2"><div><dt className="font-medium text-slate-800">任务区域</dt><dd className="m-0">{task.area_text}</dd></div><div><dt className="font-medium text-slate-800">截止时间</dt><dd className="m-0">{task.due_at}</dd></div></dl>
        <div className="mt-4 flex flex-wrap gap-2">{statusActions(task.status).map((status) => <Button key={status} size="sm" variant={status === 'completed' ? 'primary' : 'secondary'} isDisabled={busy === `status-${task.id}`} onPress={() => void changeStatus(task, status)}>{statusLabels[status]}</Button>)}<Button size="sm" variant="ghost" isDisabled={busy === `nav-${task.id}`} onPress={() => { if (!token) return; void run(`nav-${task.id}`, async () => { const result = await getTaskNavigation(token, task.id); setNavigation((current) => ({ ...current, [task.id]: result })) }, '已加载任务导航说明。') }}><Compass size={16} />导航说明</Button><Button size="sm" variant="ghost" isDisabled={busy === `safety-${task.id}`} onPress={() => { if (!token) return; void run(`safety-${task.id}`, async () => { const result = await getTaskSafetyBriefing(token, task.id); setSafety((current) => ({ ...current, [task.id]: result })) }, '已加载安全提示。') }}><ShieldCheck size={16} />安全提示</Button></div>
        {navigation[task.id] && <div className="mt-3 rounded-md border border-blue-200 bg-blue-50 p-3 text-sm text-slate-700"><strong>路线说明</strong><p className="mb-0 mt-1 leading-6">{navigation[task.id].route_summary}</p>{navigation[task.id].fallback_message && <p className="mb-0 mt-2 text-xs text-blue-800">{navigation[task.id].fallback_message}</p>}</div>}
        {safety[task.id] && <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-slate-700"><div className="flex items-center gap-2 font-semibold text-amber-900"><AlertTriangle size={16} />任务安全提示</div><ul className="mb-0 mt-2 list-disc space-y-1 pl-5">{safety[task.id].notices.map((item) => <li key={item}>{item}</li>)}</ul><p className="mb-0 mt-2 text-xs text-amber-900">{safety[task.id].emergency_stop_message}</p></div>}
        {active && <div className="mt-4 grid gap-4 border-t border-slate-200 pt-4 lg:grid-cols-2"><form className="grid gap-2" onSubmit={(event) => { event.preventDefault(); if (!token) return; const latitude = Number(taskLocation.latitude); const longitude = Number(taskLocation.longitude); const accuracy = Number(taskLocation.accuracy); if (!Number.isFinite(latitude) || !Number.isFinite(longitude) || !Number.isFinite(accuracy)) { setError('请填写有效的模拟经纬度和定位精度。'); return } void run(`location-${task.id}`, async () => { await submitTaskLocationReport(token, task.id, { source: 'simulated_demo', latitude, longitude, accuracy_meters: accuracy, captured_at: localNow() }) }, '模拟位置已上报；仅用于本任务演示，可随时停止上报。') }}><h3 className="m-0 text-sm font-semibold text-slate-950"><LocateFixed className="mr-1 inline" size={16} />模拟位置上报</h3><p className="m-0 text-xs text-slate-600">这是手动模拟演示，不请求浏览器后台定位；任务暂停、完成或取消后不可上报。</p><div className="grid grid-cols-3 gap-2"><Input aria-label="模拟纬度" type="number" value={taskLocation.latitude} onChange={(event) => setLocation((current) => ({ ...current, [task.id]: { ...taskLocation, latitude: event.target.value } }))} placeholder="纬度" /><Input aria-label="模拟经度" type="number" value={taskLocation.longitude} onChange={(event) => setLocation((current) => ({ ...current, [task.id]: { ...taskLocation, longitude: event.target.value } }))} placeholder="经度" /><Input aria-label="定位精度米" type="number" value={taskLocation.accuracy} onChange={(event) => setLocation((current) => ({ ...current, [task.id]: { ...taskLocation, accuracy: event.target.value } }))} placeholder="精度(米)" /></div><Button type="submit" size="sm" variant="secondary" isDisabled={busy === `location-${task.id}`}>上报模拟位置</Button></form><form className="grid gap-2" onSubmit={(event) => { event.preventDefault(); if (!token || !feedback[task.id]?.trim()) return; void run(`feedback-${task.id}`, async () => { await submitTaskFeedback(token, task.id, { content: feedback[task.id].trim(), occurred_at: localNow(), location_text: task.area_text, location_precision: 'approximate' }); setFeedback((current) => ({ ...current, [task.id]: '' })) }, '执行反馈已作为待审核线索提交。') }}><h3 className="m-0 text-sm font-semibold text-slate-950">执行反馈</h3><p className="m-0 text-xs text-slate-600">反馈会进入人工审核，不会自动成为确认事实。</p><TextArea aria-label="执行反馈内容" value={feedback[task.id] ?? ''} rows={3} maxLength={4000} onChange={(event) => setFeedback((current) => ({ ...current, [task.id]: event.target.value }))} fullWidth /><Button type="submit" size="sm" variant="secondary" isDisabled={busy === `feedback-${task.id}` || !feedback[task.id]?.trim()}>提交反馈</Button></form></div>}
      </article>
    })}</section>}
  </main>
}
