import { Button, Chip, Input, TextArea } from '@heroui/react'
import { AlertTriangle, Compass, LocateFixed, MapPin, RefreshCw, ShieldCheck, Users } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import {
  applyForTask, getCase, getCaseSummary, getTaskNavigation, getTaskSafetyBriefing,
  listCases, listCaseTasks, listMyTasks, listTaskCollaborationLocations,
  submitTaskFeedback, submitTaskLocationReport, updateTaskStatus,
} from '../api/cases'
import type { CaseDetail, CaseSummary, CaseTask, TaskCollaborationLocation, TaskNavigation, TaskSafetyBriefing, TaskStatus } from '../api/cases'
import { useAuth } from '../auth/useAuth'
import { EmptyState, ErrorState, LoadingState } from '../components/ContentState'

const statusLabels: Record<TaskStatus, string> = {
  pending_claim: 'Open for applications', assigned: 'Assigned', accepted: 'Accepted', active: 'Active', blocked: 'Paused', completed: 'Completed', cancelled: 'Cancelled',
}
type Failure = { message: string; retry: (() => void) | null }
type WorkspaceCase = { detail: CaseDetail; summary: CaseSummary; tasks: CaseTask[] }

function messageFrom(cause: unknown) { return cause instanceof Error ? cause.message : 'The operation could not be completed. Please try again.' }
function localNow() { return new Date().toISOString() }

export function VolunteerWorkspacePage() {
  const { token } = useAuth()
  const [cases, setCases] = useState<WorkspaceCase[]>([])
  const [myTasks, setMyTasks] = useState<CaseTask[]>([])
  const [navigation, setNavigation] = useState<Record<string, TaskNavigation>>({})
  const [safety, setSafety] = useState<Record<string, TaskSafetyBriefing>>({})
  const [locations, setLocations] = useState<Record<string, TaskCollaborationLocation[]>>({})
  const [feedback, setFeedback] = useState<Record<string, string>>({})
  const [location, setLocation] = useState<Record<string, { latitude: string; longitude: string; accuracy: string }>>({})
  const [pendingTaskIds, setPendingTaskIds] = useState<Set<string>>(new Set())
  const pendingTaskIdsRef = useRef(new Set<string>())
  const [loading, setLoading] = useState(true)
  const [failure, setFailure] = useState<Failure | null>(null)
  const [notice, setNotice] = useState('')

  const load = useCallback(async () => {
    if (!token) return
    setLoading(true); setFailure(null)
    try {
      const [memberships, assigned] = await Promise.all([listCases(token), listMyTasks(token)])
      const volunteerCases = memberships.filter((item) => item.access_role === 'volunteer')
      const workspaces = await Promise.all(volunteerCases.map(async (item) => {
        const [detail, summary, taskPage] = await Promise.all([getCase(token, item.id), getCaseSummary(token, item.id), listCaseTasks(token, item.id)])
        return { detail, summary, tasks: taskPage.items }
      }))
      setCases(workspaces); setMyTasks(assigned)
    } catch (cause) { setFailure({ message: messageFrom(cause), retry: () => void load() }) } finally { setLoading(false) }
  }, [token])
  useEffect(() => { void load() }, [load])

  async function run(taskId: string, action: () => Promise<void>, success: string) {
    if (pendingTaskIdsRef.current.has(taskId)) return
    pendingTaskIdsRef.current.add(taskId); setPendingTaskIds(new Set(pendingTaskIdsRef.current)); setFailure(null); setNotice('')
    try { await action(); setNotice(success) } catch (cause) { setFailure({ message: messageFrom(cause), retry: () => void run(taskId, action, success) }) } finally {
      pendingTaskIdsRef.current.delete(taskId); setPendingTaskIds(new Set(pendingTaskIdsRef.current))
    }
  }
  function statusActions(status: TaskStatus): TaskStatus[] {
    if (status === 'assigned') return ['accepted']
    if (status === 'accepted') return ['active']
    if (status === 'active') return ['blocked', 'completed']
    if (status === 'blocked') return ['active']
    return []
  }
  function refreshTask(updated: CaseTask) {
    setMyTasks((current) => current.map((item) => item.id === updated.id ? updated : item))
    setCases((current) => current.map((workspace) => ({ ...workspace, tasks: workspace.tasks.map((item) => item.id === updated.id ? updated : item) })))
  }
  function renderTask(task: CaseTask, assigned: boolean) {
    const busy = pendingTaskIds.has(task.id)
    const taskLocation = location[task.id] ?? { latitude: '', longitude: '', accuracy: '50' }
    return <article key={task.id} className="rounded-md border border-slate-200 bg-white p-4 shadow-sm">
      <div className="flex flex-wrap items-start justify-between gap-3"><div><h3 className="m-0 text-base font-bold text-slate-950">{task.title}</h3><p className="mb-0 mt-1 text-sm text-slate-700">{task.objective}</p></div><Chip size="sm" variant="soft"><Chip.Label>{statusLabels[task.status]}</Chip.Label></Chip></div>
      <dl className="mt-3 grid gap-2 text-sm text-slate-600 sm:grid-cols-2"><div><dt className="font-medium text-slate-800">Task area</dt><dd className="m-0">{task.area_text}</dd></div><div><dt className="font-medium text-slate-800">Due</dt><dd className="m-0">{task.due_at}</dd></div></dl>
      {!assigned && task.status !== 'completed' && task.status !== 'cancelled' && <div className="mt-3"><Button size="sm" variant="secondary" isDisabled={busy} onPress={() => { if (token) void run(task.id, async () => { await applyForTask(token, task.id) }, 'Task application submitted for commander review.') }}><Users size={16} />Apply to collaborate</Button></div>}
      {assigned && <>
        <div className="mt-4 flex flex-wrap gap-2">{statusActions(task.status).map((status) => <Button key={status} size="sm" variant={status === 'completed' ? 'primary' : 'secondary'} isDisabled={busy} onPress={() => { if (token) void run(task.id, async () => refreshTask(await updateTaskStatus(token, task.id, status)), `Task marked ${statusLabels[status]}.`) }}>{statusLabels[status]}</Button>)}
          <Button size="sm" variant="ghost" isDisabled={busy} onPress={() => { if (token) void run(task.id, async () => { const result = await getTaskNavigation(token, task.id); setNavigation((value) => ({ ...value, [task.id]: result })) }, 'Navigation instructions loaded.') }}><Compass size={16} />Navigation</Button>
          <Button size="sm" variant="ghost" isDisabled={busy} onPress={() => { if (token) void run(task.id, async () => { const result = await getTaskSafetyBriefing(token, task.id); setSafety((value) => ({ ...value, [task.id]: result })) }, 'Safety briefing loaded.') }}><ShieldCheck size={16} />Safety</Button>
          <Button size="sm" variant="ghost" isDisabled={busy} onPress={() => { if (token) void run(task.id, async () => { const result = await listTaskCollaborationLocations(token, task.id); setLocations((value) => ({ ...value, [task.id]: result })) }, 'Current collaboration locations loaded.') }}><MapPin size={16} />Collaboration locations</Button>
        </div>
        {navigation[task.id] && <p className="mt-3 rounded-md border border-blue-200 bg-blue-50 p-3 text-sm text-slate-700">{navigation[task.id].route_summary}</p>}
        {safety[task.id] && <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-slate-700"><div className="font-semibold text-amber-900"><AlertTriangle className="mr-1 inline" size={16} />Safety briefing</div><ul className="mb-0 mt-2 list-disc space-y-1 pl-5">{safety[task.id].notices.map((item, index) => <li key={`${task.id}-${index}`}>{item}</li>)}</ul></div>}
        {locations[task.id] && <ul className="mt-3 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-900">{locations[task.id].length === 0 ? <li>No current collaborator location reports.</li> : locations[task.id].map((item) => <li key={item.volunteer_user_id}>{item.volunteer_user_id}: {item.latitude.toFixed(5)}, {item.longitude.toFixed(5)} (accuracy {item.accuracy_meters}m)</li>)}</ul>}
        {task.status === 'active' && <div className="mt-4 grid gap-4 border-t border-slate-200 pt-4 lg:grid-cols-2">
          <form className="grid gap-2" onSubmit={(event) => { event.preventDefault(); if (!token) return; if (!taskLocation.latitude.trim() || !taskLocation.longitude.trim() || !taskLocation.accuracy.trim()) { setFailure({ message: 'Enter latitude, longitude, and accuracy.', retry: null }); return } const latitude = Number(taskLocation.latitude); const longitude = Number(taskLocation.longitude); const accuracy = Number(taskLocation.accuracy); if (!Number.isFinite(latitude) || latitude < -90 || latitude > 90 || !Number.isFinite(longitude) || longitude < -180 || longitude > 180 || !Number.isFinite(accuracy) || accuracy <= 0) { setFailure({ message: 'Enter valid coordinates and a positive accuracy.', retry: null }); return } const key = crypto.randomUUID(); const payload = { source: 'simulated' as const, latitude, longitude, accuracy_meters: accuracy, captured_at: localNow() }; void run(task.id, async () => { await submitTaskLocationReport(token, task.id, payload, key) }, 'Simulated location report submitted.') }}><h4 className="m-0 text-sm font-semibold text-slate-950"><LocateFixed className="mr-1 inline" size={16} />Simulated location report</h4><div className="grid grid-cols-3 gap-2"><Input aria-label="Latitude" type="number" value={taskLocation.latitude} onChange={(event) => setLocation((value) => ({ ...value, [task.id]: { ...taskLocation, latitude: event.target.value } }))} /><Input aria-label="Longitude" type="number" value={taskLocation.longitude} onChange={(event) => setLocation((value) => ({ ...value, [task.id]: { ...taskLocation, longitude: event.target.value } }))} /><Input aria-label="Accuracy in metres" type="number" value={taskLocation.accuracy} onChange={(event) => setLocation((value) => ({ ...value, [task.id]: { ...taskLocation, accuracy: event.target.value } }))} /></div><Button type="submit" size="sm" variant="secondary" isDisabled={busy}>Submit location</Button></form>
          <form className="grid gap-2" onSubmit={(event) => { event.preventDefault(); if (!token || !feedback[task.id]?.trim()) return; const key = crypto.randomUUID(); const payload = { content: feedback[task.id].trim(), occurred_at: localNow(), location_text: task.area_text, location_precision: 'approximate' as const }; void run(task.id, async () => { await submitTaskFeedback(token, task.id, payload, key); setFeedback((value) => ({ ...value, [task.id]: '' })) }, 'Feedback submitted for review.') }}><h4 className="m-0 text-sm font-semibold text-slate-950">Execution feedback</h4><TextArea aria-label="Execution feedback" value={feedback[task.id] ?? ''} rows={3} maxLength={4000} onChange={(event) => setFeedback((value) => ({ ...value, [task.id]: event.target.value }))} fullWidth /><Button type="submit" size="sm" variant="secondary" isDisabled={busy || !feedback[task.id]?.trim()}>Submit feedback</Button></form>
        </div>}
      </>}
    </article>
  }

  const myTaskIds = new Set(myTasks.map((task) => task.id))
  return <main className="mx-auto w-full max-w-6xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10"><header className="mb-7 flex flex-col items-start justify-between gap-3 sm:flex-row sm:items-end"><div><span className="mb-1 block text-xs font-semibold text-slate-500">Volunteer collaboration</span><h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">我的任务</h1><p className="mb-0 mt-1 text-sm text-slate-600">Cases, collaboration tasks, and approved task information.</p></div><Button size="sm" variant="ghost" isDisabled={loading} onPress={() => void load()}><RefreshCw size={16} />Refresh</Button></header>{notice && <p className="mb-4 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">{notice}</p>}{failure && !loading && <div className="mb-4"><ErrorState message={failure.message} onRetry={failure.retry ?? undefined} /></div>}{loading ? <LoadingState label="Loading your collaboration workspace" /> : cases.length === 0 ? <EmptyState icon={Users} title="No collaboration cases yet" description="Cases where you are added as a volunteer will appear here before a task is assigned." /> : <section className="grid gap-6">{cases.map((workspace) => <article key={workspace.detail.id} className="border-t border-slate-300 pt-5"><div className="flex flex-wrap items-start justify-between gap-3"><div><span className="text-xs font-semibold text-slate-500">{workspace.detail.case_code}</span><h2 className="m-0 text-xl font-bold text-slate-950">{workspace.detail.elder_profile.display_name}</h2></div><Chip size="sm" variant="soft"><Chip.Label>{workspace.detail.status}</Chip.Label></Chip></div><div className="mt-3 grid gap-4 text-sm text-slate-700 lg:grid-cols-3"><div><h3 className="m-0 text-sm font-semibold text-slate-950">Family contact</h3><p className="mb-0 mt-1">{workspace.detail.family_contact_emails?.join(', ') || 'No family contact listed.'}</p></div><div><h3 className="m-0 text-sm font-semibold text-slate-950">Health notes</h3><p className="mb-0 mt-1">{workspace.detail.elder_profile.health_notes || 'No health notes listed.'}</p></div><div><h3 className="m-0 text-sm font-semibold text-slate-950">Current focus</h3><p className="mb-0 mt-1">{workspace.summary.current_focus.map((item) => item.detail).join(' ') || 'No current focus.'}</p></div></div><div className="mt-4 grid gap-4 lg:grid-cols-2"><section><h3 className="m-0 text-sm font-semibold text-slate-950">Visible clues</h3><ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-slate-700">{workspace.detail.clues.slice(0, 5).map((clue) => <li key={clue.id}>{clue.content}</li>)}</ul></section><section><h3 className="m-0 text-sm font-semibold text-slate-950">Tasks</h3><div className="mt-2 grid gap-3">{workspace.tasks.map((task) => renderTask(myTasks.find((item) => item.id === task.id) ?? task, myTaskIds.has(task.id)))}</div></section></div></article>)}</section>}</main>
}
