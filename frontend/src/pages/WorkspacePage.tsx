import { Card, Chip } from '@heroui/react'
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
    <div className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 min-h-14">
        <span className="mb-1 block text-xs font-semibold text-slate-500">{context}</span>
        <h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">{title}</h1>
      </header>

      <Card className="overflow-hidden rounded-md! border border-slate-200 shadow-none" aria-labelledby="workspace-title">
        <Card.Header className="flex min-h-18 flex-row! items-center! justify-between! gap-5 border-b border-slate-200 px-5 py-4">
          <Card.Title id="workspace-title" className="text-base font-bold text-slate-950">当前工作</Card.Title>
          <Chip size="sm" variant="soft">
            <Chip.Label>0 条记录</Chip.Label>
          </Chip>
        </Card.Header>
        <Card.Content className="flex min-h-64 flex-col items-center justify-center px-5 py-8 text-center">
          <span className="grid size-10 place-items-center rounded-md bg-slate-100 text-slate-400" aria-hidden="true">
            <Inbox size={22} />
          </span>
          <strong className="mt-4 text-sm text-slate-950">{emptyTitle}</strong>
          <span className="mt-1 max-w-md text-xs text-slate-500">{emptyDescription}</span>
        </Card.Content>
      </Card>
    </div>
  )
}
