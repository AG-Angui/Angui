import { useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import {
  getCurrentUser,
  login as requestLogin,
  logout as requestLogout,
} from '../api/auth'
import type { AuthUser } from '../api/auth'
import { AuthContext } from './auth-context'
import type { AuthContextValue } from './auth-context'

const TOKEN_KEY = 'angui.session.token'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(() => sessionStorage.getItem(TOKEN_KEY))
  const [user, setUser] = useState<AuthUser | null>(null)
  const [isLoading, setIsLoading] = useState(Boolean(token))

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
        sessionStorage.removeItem(TOKEN_KEY)
        setToken(null)
        setUser(null)
      })
      .finally(() => {
        if (active) setIsLoading(false)
      })

    return () => {
      active = false
    }
  }, [token])

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
          sessionStorage.removeItem(TOKEN_KEY)
          setToken(null)
          setUser(null)
        }
      },
    }),
    [isLoading, token, user],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
