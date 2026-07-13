import { Spinner } from '@heroui/react'
import { Navigate, Route, Routes } from 'react-router-dom'
import type { ReactNode } from 'react'
import type { UserRole } from './api/auth'
import { useAuth } from './auth/useAuth'
import { AppShell } from './components/AppShell'
import { CaseWorkspacePage } from './pages/CaseWorkspacePage'
import { DashboardPage } from './pages/DashboardPage'
import { LoginPage } from './pages/LoginPage'

function RoleRoute({ role, children }: { role: UserRole; children: ReactNode }) {
  const { user } = useAuth()
  return user?.role === role ? children : <Navigate to="/" replace />
}

function App() {
  const { user, isLoading } = useAuth()

  if (isLoading) {
    return (
      <main className="grid min-h-screen place-items-center bg-canvas" aria-label="正在恢复会话">
        <Spinner size="lg" />
      </main>
    )
  }

  if (!user) {
    return <LoginPage />
  }

  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<DashboardPage />} />
        <Route
          path="family"
          element={
            <RoleRoute role="family">
              <CaseWorkspacePage mode="family" />
            </RoleRoute>
          }
        />
        <Route
          path="command"
          element={
            <RoleRoute role="commander">
              <CaseWorkspacePage mode="commander" />
            </RoleRoute>
          }
        />
        <Route
          path="volunteer"
          element={
            <RoleRoute role="volunteer">
              <CaseWorkspacePage mode="volunteer" />
            </RoleRoute>
          }
        />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}

export default App
