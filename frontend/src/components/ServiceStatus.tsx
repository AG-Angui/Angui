import { CircleAlert, CircleCheck, LoaderCircle, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { getHealth, type HealthResponse } from '../api/health'

type RequestState =
  | { status: 'checking' }
  | { status: 'online'; health: HealthResponse }
  | { status: 'offline' }

interface ServiceStatusProps {
  compact?: boolean
}

export function ServiceStatus({ compact = false }: ServiceStatusProps) {
  const [state, setState] = useState<RequestState>({ status: 'checking' })

  const checkHealth = useCallback(async () => {
    const controller = new AbortController()
    const timeoutId = window.setTimeout(() => controller.abort(), 4000)
    setState({ status: 'checking' })

    try {
      const health = await getHealth(controller.signal)
      setState({ status: 'online', health })
    } catch {
      setState({ status: 'offline' })
    } finally {
      window.clearTimeout(timeoutId)
    }
  }, [])

  useEffect(() => {
    void checkHealth()
  }, [checkHealth])

  const content = {
    checking: {
      icon: <LoaderCircle className="status-spinner" size={16} aria-hidden="true" />,
      label: '正在连接后端',
      className: 'service-checking',
    },
    online: {
      icon: <CircleCheck size={16} aria-hidden="true" />,
      label: `服务在线${state.status === 'online' && !compact ? ` · v${state.health.version}` : ''}`,
      className: 'service-online',
    },
    offline: {
      icon: <CircleAlert size={16} aria-hidden="true" />,
      label: '后端未连接',
      className: 'service-offline',
    },
  }[state.status]

  return (
    <div className={`service-status ${content.className}`} role="status">
      {content.icon}
      <span>{content.label}</span>
      {state.status === 'offline' && (
        <button
          type="button"
          className="icon-button"
          onClick={() => void checkHealth()}
          aria-label="重新检查后端连接"
          title="重新检查后端连接"
        >
          <RefreshCw size={15} aria-hidden="true" />
        </button>
      )}
    </div>
  )
}
