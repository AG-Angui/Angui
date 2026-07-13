import {
  ClipboardCheck,
  FileSearch,
  RadioTower,
  UsersRound,
} from 'lucide-react'
import { ServiceStatus } from '../components/ServiceStatus'

const metrics = [
  { label: '活动案件', value: 0, icon: RadioTower, tone: 'red' },
  { label: '待审核线索', value: 0, icon: FileSearch, tone: 'amber' },
  { label: '执行中任务', value: 0, icon: ClipboardCheck, tone: 'blue' },
  { label: '在线志愿者', value: 0, icon: UsersRound, tone: 'green' },
]

export function DashboardPage() {
  return (
    <div className="page">
      <header className="page-header">
        <div>
          <span className="page-context">协同工作台</span>
          <h1>行动总览</h1>
        </div>
        <ServiceStatus />
      </header>

      <section className="metrics-grid" aria-label="行动指标">
        {metrics.map(({ label, value, icon: Icon, tone }) => (
          <article className="metric-card" key={label}>
            <span className={`metric-icon metric-${tone}`} aria-hidden="true">
              <Icon size={19} />
            </span>
            <div>
              <span>{label}</span>
              <strong>{value}</strong>
            </div>
          </article>
        ))}
      </section>

      <section className="activity-section" aria-labelledby="case-status-title">
        <div className="section-heading">
          <div>
            <span className="page-context">实时状态</span>
            <h2 id="case-status-title">案件态势</h2>
          </div>
          <span className="record-count">0 条记录</span>
        </div>
        <div className="empty-state">
          <RadioTower size={24} aria-hidden="true" />
          <strong>当前没有活动案件</strong>
          <span>新的授权案件建立后会出现在这里。</span>
        </div>
      </section>
    </div>
  )
}
