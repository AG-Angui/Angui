import { Button, Chip, Spinner, Tooltip } from '@heroui/react'
import { CircleAlert, CircleCheck, RefreshCw } from 'lucide-react'
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

  if (state.status === 'checking') {
    return (
      <Chip color="accent" size="sm" variant="soft">
        <Spinner size="sm" color="accent" aria-hidden="true" />
        <Chip.Label>正在连接后端</Chip.Label>
      </Chip>
    )
  }

  if (state.status === 'online') {
    return (
      <Chip color="success" size="sm" variant="soft">
        <CircleCheck size={15} aria-hidden="true" />
        <Chip.Label>
          服务在线{compact ? '' : ` · v${state.health.version}`}
        </Chip.Label>
      </Chip>
    )
  }

  return (
    <div className="flex items-center gap-1.5" role="status">
      <Chip color="danger" size="sm" variant="soft">
        <CircleAlert size={15} aria-hidden="true" />
        <Chip.Label>后端未连接</Chip.Label>
      </Chip>
      <Tooltip delay={300}>
        <Button
          isIconOnly
          size="sm"
          variant="ghost"
          onPress={() => void checkHealth()}
          aria-label="重新检查后端连接"
        >
          <RefreshCw size={15} aria-hidden="true" />
        </Button>
        <Tooltip.Content>重新检查后端连接</Tooltip.Content>
      </Tooltip>
    </div>
  )
}
