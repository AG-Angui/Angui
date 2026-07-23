import { Button, Spinner } from '@heroui/react'
import { CircleAlert, Inbox, RefreshCw, type LucideIcon } from 'lucide-react'

interface Action {
  label: string
  onPress: () => void
}

interface EmptyStateProps {
  title: string
  description?: string
  icon?: LucideIcon
  action?: Action
}

interface ErrorStateProps {
  message: string
  onRetry?: () => void
}

export function LoadingState({ label = '正在加载' }: { label?: string }) {
  return (
    <div className="flex min-h-40 flex-col items-center justify-center gap-3 px-5 py-8" role="status">
      <Spinner />
      <span className="text-sm text-slate-500">{label}</span>
    </div>
  )
}

export function EmptyState({
  title,
  description,
  icon: Icon = Inbox,
  action,
}: EmptyStateProps) {
  return (
    <div className="flex min-h-48 flex-col items-center justify-center px-5 py-8 text-center">
      <span className="grid size-10 place-items-center rounded-md bg-slate-100 text-slate-400" aria-hidden="true">
        <Icon size={22} />
      </span>
      <strong className="mt-4 text-sm text-slate-950">{title}</strong>
      {description && <span className="mt-1 max-w-md text-xs leading-5 text-slate-500">{description}</span>}
      {action && (
        <Button className="mt-4" size="sm" variant="secondary" onPress={action.onPress}>
          {action.label}
        </Button>
      )}
    </div>
  )
}

export function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div className="flex min-h-48 flex-col items-center justify-center px-5 py-8 text-center" role="alert">
      <span className="grid size-10 place-items-center rounded-md bg-red-50 text-red-700" aria-hidden="true">
        <CircleAlert size={22} />
      </span>
      <strong className="mt-4 text-sm text-slate-950">暂时无法加载内容</strong>
      <span className="mt-1 max-w-md text-xs leading-5 text-slate-500">{message}</span>
      {onRetry && (
        <Button className="mt-4" size="sm" variant="secondary" onPress={onRetry}>
          <RefreshCw size={15} aria-hidden="true" />
          重试
        </Button>
      )}
    </div>
  )
}
