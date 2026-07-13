import { Button, Card, Chip, Input } from '@heroui/react'
import { LockKeyhole, LogIn, ShieldCheck } from 'lucide-react'
import { useState } from 'react'
import brandMark from '../../../assets/brand/angui-mark.svg'
import { useAuth } from '../auth/useAuth'

export function LoginPage() {
  const { login } = useAuth()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError('')
    setIsSubmitting(true)
    try {
      await login(email, password)
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '登录失败')
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <main className="grid min-h-screen place-items-center bg-canvas px-4 py-8">
      <div className="w-full max-w-sm">
        <div className="mb-5 flex items-center gap-3 px-1">
          <img src={brandMark} alt="安归" className="size-12 rounded-md" />
          <div>
            <h1 className="m-0 text-2xl font-bold text-slate-950">安归</h1>
            <p className="m-0 mt-0.5 text-sm text-slate-500">协同工作台</p>
          </div>
        </div>

        <Card className="rounded-md! border border-slate-200 shadow-none">
          <Card.Header className="border-b border-slate-200 px-5 py-4">
            <div className="flex w-full items-center justify-between gap-4">
              <div>
                <Card.Title className="text-base font-bold text-slate-950">账号登录</Card.Title>
                <p className="m-0 mt-1 text-xs text-slate-500">使用已批准的项目账号进入</p>
              </div>
              <Chip size="sm" variant="soft">
                <ShieldCheck size={14} aria-hidden="true" />
                <Chip.Label>受控访问</Chip.Label>
              </Chip>
            </div>
          </Card.Header>
          <Card.Content className="p-5">
            <form className="space-y-4" onSubmit={handleSubmit}>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold text-slate-600">邮箱</span>
                <Input
                  type="email"
                  autoComplete="username"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  fullWidth
                  required
                  placeholder="name@example.com"
                />
              </label>
              <label className="block">
                <span className="mb-1.5 block text-xs font-semibold text-slate-600">密码</span>
                <Input
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  fullWidth
                  required
                />
              </label>

              {error && (
                <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700" role="alert">
                  {error}
                </div>
              )}

              <Button type="submit" variant="primary" fullWidth isDisabled={isSubmitting}>
                {isSubmitting ? <LockKeyhole size={17} /> : <LogIn size={17} />}
                {isSubmitting ? '正在验证' : '登录'}
              </Button>
            </form>
          </Card.Content>
        </Card>
      </div>
    </main>
  )
}
