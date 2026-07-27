import { Button, Input, Spinner } from '@heroui/react'
import { Save, UserRound } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { getMyProfile, updateMyProfile } from '../api/auth'
import type { UserProfile } from '../api/auth'
import { ApiClientError } from '../api/client'
import { useAuth } from '../auth/useAuth'
import { ErrorState, LoadingState } from '../components/ContentState'

function messageFrom(cause: unknown) {
  return cause instanceof ApiClientError ? cause.message : '暂时无法完成操作，请稍后重试。'
}

export function ProfilePage() {
  const { token, refreshUser } = useAuth()
  const [profile, setProfile] = useState<UserProfile | null>(null)
  const [displayName, setDisplayName] = useState('')
  const [avatarReference, setAvatarReference] = useState('')
  const [locale, setLocale] = useState<'zh-CN' | 'en-US'>('zh-CN')
  const [reducedMotion, setReducedMotion] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const load = useCallback(async () => {
    if (!token) return
    setIsLoading(true)
    setError('')
    try {
      const next = await getMyProfile(token)
      setProfile(next)
      setDisplayName(next.display_name)
      setAvatarReference(next.avatar_reference ?? '')
      setLocale(next.preferences.locale)
      setReducedMotion(next.preferences.reduced_motion)
    } catch (cause) {
      setError(messageFrom(cause))
    } finally {
      setIsLoading(false)
    }
  }, [token])

  useEffect(() => { void load() }, [load])

  if (isLoading) return <LoadingState label="正在加载个人资料" />
  if (!profile) return <ErrorState message={error || '个人资料不可用'} onRetry={() => void load()} />

  return (
    <main className="mx-auto w-full max-w-3xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-6 flex items-start gap-3">
        <span className="grid size-11 place-items-center rounded-full bg-brand-100 text-brand-700"><UserRound aria-hidden="true" /></span>
        <div>
          <h1 className="m-0 text-2xl font-bold text-slate-950">个人资料</h1>
          <p className="mb-0 mt-1 text-sm text-slate-600">更新显示名称、头像引用和非敏感界面偏好。</p>
        </div>
      </header>
      <form className="space-y-5 rounded-lg border border-slate-200 bg-white p-5 shadow-sm sm:p-6" onSubmit={(event) => {
        event.preventDefault()
        if (!token) return
        setIsSaving(true); setError(''); setNotice('')
        void updateMyProfile(token, { display_name: displayName, avatar_reference: avatarReference, preferences: { locale, reduced_motion: reducedMotion } })
          .then(async (next) => {
            setProfile(next)
            setNotice('个人资料已保存。')
            try {
              await refreshUser()
            } catch {
              // Keeping the locally returned profile is sufficient when the best-effort identity refresh fails.
            }
          })
          .catch((cause) => setError(messageFrom(cause)))
          .finally(() => setIsSaving(false))
      }}>
        <div className="grid gap-4 sm:grid-cols-2">
          <ProfileField label="显示名称"><Input value={displayName} onChange={(event) => setDisplayName(event.target.value)} maxLength={120} fullWidth required /></ProfileField>
          <ProfileField label="账号邮箱" hint="邮箱、账号类型和权限不能在此页面修改。"><Input value={profile.email} readOnly fullWidth /></ProfileField>
          <div className="sm:col-span-2"><ProfileField label="头像引用" hint="填写已受控存储中的头像引用；留空可清除。"><Input value={avatarReference} onChange={(event) => setAvatarReference(event.target.value)} maxLength={500} fullWidth /></ProfileField></div>
          <label className="grid gap-1 text-sm font-medium text-slate-700">界面语言
            <select className="min-h-10 rounded-md border border-slate-300 bg-white px-3 text-sm" value={locale} onChange={(event) => setLocale(event.target.value as 'zh-CN' | 'en-US')}>
              <option value="zh-CN">简体中文</option><option value="en-US">English (US)</option>
            </select>
          </label>
          <label className="flex min-h-10 items-center gap-2 self-end text-sm text-slate-700"><input type="checkbox" checked={reducedMotion} onChange={(event) => setReducedMotion(event.target.checked)} /> 减少界面动画</label>
        </div>
        {error && <p className="m-0 rounded-md bg-danger-50 px-3 py-2 text-sm text-danger-700" role="alert">{error}</p>}
        {notice && <p className="m-0 rounded-md bg-success-50 px-3 py-2 text-sm text-success-700" role="status">{notice}</p>}
        <div className="flex justify-end"><Button type="submit" variant="primary" isDisabled={isSaving}>{isSaving ? <Spinner size="sm" /> : <Save size={16} aria-hidden="true" />}{isSaving ? '正在保存' : '保存资料'}</Button></div>
      </form>
    </main>
  )
}

function ProfileField({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return <label className="grid gap-1 text-sm font-medium text-slate-700"><span>{label}</span>{children}{hint && <span className="text-xs font-normal text-slate-500">{hint}</span>}</label>
}
