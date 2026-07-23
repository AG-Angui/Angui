import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import {
  getCurrentUser,
  login as requestLogin,
  logout as requestLogout,
} from '../api/auth'
import type { AuthUser } from '../api/auth'
import { SESSION_EXPIRED_EVENT } from '../api/client'
import { AuthContext } from './auth-context'
import type { AuthContextValue } from './auth-context'

const TOKEN_KEY = 'angui.session.token'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(() => sessionStorage.getItem(TOKEN_KEY))
  const [user, setUser] = useState<AuthUser | null>(null)
  const [isLoading, setIsLoading] = useState(Boolean(token))
  const clearSession = useCallback(() => {
    sessionStorage.removeItem(TOKEN_KEY)
    setToken(null)
    setUser(null)
  }, [])

  useEffect(() => {
    window.addEventListener(SESSION_EXPIRED_EVENT, clearSession)
    return () => window.removeEventListener(SESSION_EXPIRED_EVENT, clearSession)
  }, [clearSession])

  useEffect(() => {
    if (!token) {
      setUser(null)
      setIsLoading(false)
      return
    }

    let active = true
    setIsLoading(true)
    getCurrentUser(token)
      .then((currentUser) => {
        if (active) setUser(currentUser)
      })
      .catch(() => {
        if (!active) return
        clearSession()
      })
      .finally(() => {
        if (active) setIsLoading(false)
      })

    return () => {
      active = false
    }
  }, [clearSession, token])

  const value = useMemo<AuthContextValue>(
    () => ({
      user,
      token,
      isLoading,
      login: async (email, password) => {
        const response = await requestLogin(email, password)
        sessionStorage.setItem(TOKEN_KEY, response.token)
        setToken(response.token)
        setUser(response.user)
      },
      logout: async () => {
        if (!token) return
        try {
          await requestLogout(token)
        } finally {
          clearSession()
        }
      },
    }),
    [clearSession, isLoading, token, user],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
