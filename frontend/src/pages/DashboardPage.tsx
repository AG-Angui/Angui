import { Card, Chip } from '@heroui/react'
import { ClipboardCheck, FileSearch, RadioTower, UsersRound } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { getCase, listCases } from '../api/cases'
import type { CaseDetail, CaseListItem } from '../api/cases'
import { useAuth } from '../auth/useAuth'
import { ServiceStatus } from '../components/ServiceStatus'

export function DashboardPage() {
  const { token, user } = useAuth()
  const [cases, setCases] = useState<CaseListItem[]>([])
  const [details, setDetails] = useState<CaseDetail[]>([])

  useEffect(() => {
    if (!token) return
    listCases(token)
      .then(async (items) => {
        setCases(items)
        const loaded = await Promise.all(items.slice(0, 20).map((item) => getCase(token, item.id)))
        setDetails(loaded)
      })
      .catch(() => {
        setCases([])
        setDetails([])
      })
  }, [token])

  const pendingClues = useMemo(
    () => details.flatMap((item) => item.clues).filter((clue) => clue.status === 'pending_review').length,
    [details],
  )
  const activeCases = cases.filter((item) => item.status === 'active').length
  const confirmedClues = details.flatMap((item) => item.clues).filter((clue) => clue.status === 'confirmed').length

  const metrics = [
    { label: '活动案件', value: activeCases, icon: RadioTower, iconClass: 'bg-red-50 text-red-700' },
    { label: '待审核线索', value: pendingClues, icon: FileSearch, iconClass: 'bg-amber-50 text-amber-700' },
    { label: '已确认线索', value: confirmedClues, icon: ClipboardCheck, iconClass: 'bg-blue-50 text-blue-700' },
    { label: '可访问案件', value: cases.length, icon: UsersRound, iconClass: 'bg-emerald-50 text-emerald-700' },
  ]

  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 flex min-h-14 flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <span className="mb-1 block text-xs font-semibold text-slate-500">{user?.display_name}</span>
          <h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">行动总览</h1>
        </div>
        <ServiceStatus />
      </header>

      <section className="mb-7 grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4 lg:gap-3" aria-label="行动指标">
        {metrics.map(({ label, value, icon: Icon, iconClass }) => (
          <Card key={label} className="min-h-24 rounded-md! border border-slate-200 shadow-none">
            <Card.Content className="flex h-full flex-row! items-center! gap-3 p-4">
              <span className={`grid size-10 shrink-0 place-items-center rounded-md ${iconClass}`} aria-hidden="true">
                <Icon size={19} />
              </span>
              <div className="min-w-0">
                <span className="block whitespace-nowrap text-xs text-slate-500">{label}</span>
                <strong className="mt-1 block text-2xl leading-none text-slate-950">{value}</strong>
              </div>
            </Card.Content>
          </Card>
        ))}
      </section>

      <section className="overflow-hidden border-y border-slate-200 bg-white" aria-labelledby="case-status-title">
        <header className="flex min-h-18 items-center justify-between gap-5 border-b border-slate-200 px-5 py-4">
          <div>
            <span className="mb-0.5 block text-xs font-semibold text-slate-500">实时状态</span>
            <h2 id="case-status-title" className="m-0 text-base font-bold text-slate-950">案件态势</h2>
          </div>
          <Chip size="sm" variant="soft">
            <Chip.Label>{cases.length} 条记录</Chip.Label>
          </Chip>
        </header>
        {cases.length === 0 ? (
          <div className="flex min-h-56 flex-col items-center justify-center px-5 py-8 text-center">
            <span className="grid size-10 place-items-center rounded-md bg-slate-100 text-slate-400" aria-hidden="true">
              <RadioTower size={22} />
            </span>
            <strong className="mt-4 text-sm text-slate-950">当前没有可访问案件</strong>
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {cases.slice(0, 6).map((item) => (
              <div key={item.id} className="grid gap-2 px-5 py-3 sm:grid-cols-[140px_minmax(0,1fr)_auto] sm:items-center">
                <strong className="text-sm text-slate-950">{item.case_code}</strong>
                <span className="truncate text-sm text-slate-600">{item.display_name} · {item.last_seen_location ?? '地点待补充'}</span>
                <Chip size="sm" variant="soft"><Chip.Label>{item.status}</Chip.Label></Chip>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  )
}
