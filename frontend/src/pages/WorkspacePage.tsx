import { Inbox } from 'lucide-react'

interface WorkspacePageProps {
  context: string
  title: string
  emptyTitle: string
  emptyDescription: string
}

export function WorkspacePage({
  context,
  title,
  emptyTitle,
  emptyDescription,
}: WorkspacePageProps) {
  return (
    <div className="page">
      <header className="page-header">
        <div>
          <span className="page-context">{context}</span>
          <h1>{title}</h1>
        </div>
      </header>

      <section className="activity-section" aria-labelledby="workspace-title">
        <div className="section-heading">
          <h2 id="workspace-title">当前工作</h2>
          <span className="record-count">0 条记录</span>
        </div>
        <div className="empty-state">
          <Inbox size={24} aria-hidden="true" />
          <strong>{emptyTitle}</strong>
          <span>{emptyDescription}</span>
        </div>
      </section>
    </div>
  )
}
