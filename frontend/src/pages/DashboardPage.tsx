import { Card, Chip } from '@heroui/react'
import {
  ClipboardCheck,
  FileSearch,
  RadioTower,
  UsersRound,
} from 'lucide-react'
import { ServiceStatus } from '../components/ServiceStatus'

const metrics = [
  {
    label: '活动案件',
    value: 0,
    icon: RadioTower,
    iconClass: 'bg-red-50 text-red-700',
  },
  {
    label: '待审核线索',
    value: 0,
    icon: FileSearch,
    iconClass: 'bg-amber-50 text-amber-700',
  },
  {
    label: '执行中任务',
    value: 0,
    icon: ClipboardCheck,
    iconClass: 'bg-blue-50 text-blue-700',
  },
  {
    label: '在线志愿者',
    value: 0,
    icon: UsersRound,
    iconClass: 'bg-emerald-50 text-emerald-700',
  },
]

export function DashboardPage() {
  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 flex min-h-14 flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <span className="mb-1 block text-xs font-semibold text-slate-500">协同工作台</span>
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

      <Card className="overflow-hidden rounded-md! border border-slate-200 shadow-none" aria-labelledby="case-status-title">
        <Card.Header className="flex min-h-18 flex-row! items-center! justify-between! gap-5 border-b border-slate-200 px-5 py-4">
          <div>
            <span className="mb-0.5 block text-xs font-semibold text-slate-500">实时状态</span>
            <Card.Title id="case-status-title" className="text-base font-bold text-slate-950">案件态势</Card.Title>
          </div>
          <Chip size="sm" variant="soft">
            <Chip.Label>0 条记录</Chip.Label>
          </Chip>
        </Card.Header>
        <Card.Content className="flex min-h-64 flex-col items-center justify-center px-5 py-8 text-center">
          <span className="grid size-10 place-items-center rounded-md bg-slate-100 text-slate-400" aria-hidden="true">
            <RadioTower size={22} />
          </span>
          <strong className="mt-4 text-sm text-slate-950">当前没有活动案件</strong>
          <span className="mt-1 max-w-md text-xs text-slate-500">新的授权案件建立后会出现在这里。</span>
        </Card.Content>
      </Card>
    </div>
  )
}
