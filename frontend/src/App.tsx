import { Spinner } from '@heroui/react'
import { Navigate, Route, Routes } from 'react-router-dom'
import type { ReactNode } from 'react'
import { useAuth } from './auth/useAuth'
import { AppShell } from './components/AppShell'
import { CaseWorkspacePage } from './pages/CaseWorkspacePage'
import { DashboardPage } from './pages/DashboardPage'
import { LoginPage } from './pages/LoginPage'

function CaseRoleRoute({ children }: { children: ReactNode }) {
  const { user } = useAuth()
  return user && ['family', 'commander', 'volunteer'].includes(user.global_role)
    ? children
    : <Navigate to="/" replace />
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
            <CaseRoleRoute>
              <CaseWorkspacePage mode="family" />
            </CaseRoleRoute>
          }
        />
        <Route
          path="command"
          element={
            <CaseRoleRoute>
              <CaseWorkspacePage mode="commander" />
            </CaseRoleRoute>
          }
        />
        <Route
          path="volunteer"
          element={
            <CaseRoleRoute>
              <CaseWorkspacePage mode="volunteer" />
            </CaseRoleRoute>
          }
        />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}

export default App
