import { useCallback, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  getCurrentUser,
  login as requestLogin,
  logout as requestLogout,
} from "../api/auth";
import type { AuthUser } from "../api/auth";
import { SESSION_EXPIRED_EVENT } from "../api/client";
import { AuthContext } from "./auth-context";
import type { AuthContextValue } from "./auth-context";

const TOKEN_KEY = "angui.session.token";

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(() =>
    sessionStorage.getItem(TOKEN_KEY),
  );
  const [user, setUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState(Boolean(token));
  const [isLoggingOut, setIsLoggingOut] = useState(false);
  const [sessionNotice, setSessionNotice] = useState<string | null>(null);
  const clearSession = useCallback((notice?: string) => {
    sessionStorage.removeItem(TOKEN_KEY);
    setToken(null);
    setUser(null);
    if (notice) setSessionNotice(notice);
  }, []);

  useEffect(() => {
    const handleSessionExpired = () => {
      clearSession("登录状态已失效，请重新登录。");
    };
    window.addEventListener(SESSION_EXPIRED_EVENT, handleSessionExpired);
    return () =>
      window.removeEventListener(SESSION_EXPIRED_EVENT, handleSessionExpired);
  }, [clearSession]);

  useEffect(() => {
    if (!token) {
      // Session clearing is handled synchronously by clearSession(). Do not
      // reset user here: this effect can be queued from the initial null-token
      // render and run after login has already populated the user state.
      setIsLoading(false);
      return;
    }

    let active = true;
    setIsLoading(true);
    getCurrentUser(token)
      .then((currentUser) => {
        if (active) setUser(currentUser);
      })
      .catch(() => {
        if (!active) return;
        clearSession();
      })
      .finally(() => {
        if (active) setIsLoading(false);
      });

    return () => {
      active = false;
    };
  }, [clearSession, token]);

  const value = useMemo<AuthContextValue>(
    () => ({
      user,
      token,
      isLoading,
      isLoggingOut,
      sessionNotice,
      login: async (email, password) => {
        const response = await requestLogin(email, password);
        sessionStorage.setItem(TOKEN_KEY, response.token);
        setSessionNotice(null);
        setToken(response.token);
        setUser(response.user);
      },
      logout: async () => {
        if (!token) return;
        setIsLoggingOut(true);
        try {
          await requestLogout(token);
        } catch {
          // Remote revocation is best effort; always complete the local safety exit.
        } finally {
          clearSession();
          setIsLoggingOut(false);
        }
      },
      refreshUser: async () => {
        if (!token) return;
        const currentUser = await getCurrentUser(token);
        setUser(currentUser);
      },
    }),
    [clearSession, isLoading, isLoggingOut, sessionNotice, token, user],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
