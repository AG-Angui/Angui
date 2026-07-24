import { Button, Card, Chip } from '@heroui/react'
import { ClipboardCheck, FileSearch, RadioTower, RefreshCw, UsersRound } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { getCase, listCases } from '../api/cases'
import type { CaseDetail, CaseListItem } from '../api/cases'
import { useAuth } from '../auth/useAuth'
import { EmptyState, ErrorState, LoadingState } from '../components/ContentState'
import { ServiceStatus } from '../components/ServiceStatus'

const caseStatusLabels: Record<string, string> = {
  active: '进行中',
  resolved: '已找到',
  closed: '已关闭',
}

export function DashboardPage() {
  const { token, user } = useAuth()
  const [cases, setCases] = useState<CaseListItem[]>([])
  const [details, setDetails] = useState<CaseDetail[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState('')

  const loadDashboard = useCallback(async () => {
    if (!token) return
    setIsLoading(true)
    setError('')
    try {
      const items = await listCases(token)
      const settledDetails = await Promise.allSettled(
        items.slice(0, 20).map((item) => getCase(token, item.id)),
      )
      const loaded = settledDetails.flatMap((result) =>
        result.status === 'fulfilled' ? [result.value] : [],
      )

      setCases(items)
      setDetails(loaded)
      if (loaded.length !== items.length) {
        setError('部分案件详情暂时不可用，统计数据可能不完整。')
      }
    } catch (cause) {
      setCases([])
      setDetails([])
      setError(cause instanceof Error ? cause.message : '无法连接案件服务。')
    } finally {
      setIsLoading(false)
    }
  }, [token])

  useEffect(() => {
    void loadDashboard()
  }, [loadDashboard])

  const pendingClues = useMemo(
    () => details.flatMap((item) => item.clues).filter((clue) => clue.status === 'pending_review').length,
    [details],
  )
  const activeCases = cases.filter((item) => item.status === 'active').length
  const confirmedClues = details.flatMap((item) => item.clues).filter((clue) => clue.status === 'confirmed').length
  const emptyState = user?.account_type === 'learner'
    ? {
        title: '新人账号暂未获得案件权限',
        description: '当前后端只会返回你作为案件成员可访问的案件；学习模块尚未提供接口。',
      }
    : user?.global_capabilities.includes('admin')
      ? {
          title: '管理员账号不自动拥有案件权限',
          description: '管理员需要先被授予具体案件成员关系，才能查看案件内容。',
        }
      : {
          title: '当前没有可访问案件',
          description: '创建案件或由案件成员邀请后，案件会显示在这里。',
        }

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
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="ghost"
            isDisabled={isLoading}
            onPress={() => void loadDashboard()}
          >
            <RefreshCw size={16} aria-hidden="true" />
            刷新
          </Button>
          <ServiceStatus />
        </div>
      </header>

      {error && !isLoading && cases.length > 0 && (
        <div className="mb-5 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800" role="status">
          {error}
        </div>
      )}

      {isLoading ? (
        <section className="border-y border-slate-200 bg-white" aria-label="正在加载行动总览">
          <LoadingState label="正在加载案件和线索统计" />
        </section>
      ) : error && cases.length === 0 ? (
        <section className="border-y border-slate-200 bg-white" aria-label="行动总览加载失败">
          <ErrorState message={error} onRetry={() => void loadDashboard()} />
        </section>
      ) : (
        <>
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
          <EmptyState icon={RadioTower} {...emptyState} />
        ) : (
          <div className="divide-y divide-slate-100">
            {cases.slice(0, 6).map((item) => (
              <div key={item.id} className="grid gap-2 px-5 py-3 sm:grid-cols-[140px_minmax(0,1fr)_auto] sm:items-center">
                <strong className="text-sm text-slate-950">{item.case_code}</strong>
                <span className="truncate text-sm text-slate-600">{item.display_name} · {item.last_seen_location ?? '地点待补充'}</span>
                <Chip size="sm" variant="soft"><Chip.Label>{caseStatusLabels[item.status] ?? item.status}</Chip.Label></Chip>
              </div>
            ))}
          </div>
        )}
      </section>
        </>
      )}
    </div>
  )
}
