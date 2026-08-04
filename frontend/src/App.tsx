import { Spinner } from "@heroui/react";
import { Navigate, Route, Routes } from "react-router";
import type { ReactNode } from "react";
import { useAuth } from "./auth/useAuth";
import { AppShell } from "./components/AppShell";
import { CaseWorkspacePage } from "./pages/CaseWorkspacePage";
import { DashboardPage } from "./pages/DashboardPage";
import { LoginPage } from "./pages/LoginPage";
import { LearningCenterPage } from "./pages/LearningCenterPage";
import { LearningGovernancePage } from "./pages/LearningGovernancePage";
import { ProfilePage } from "./pages/ProfilePage";
import { VolunteerWorkspacePage } from "./pages/VolunteerWorkspacePage";

function CaseRoleRoute({
  capability,
  familyOnly,
  children,
}: {
  capability?: "commander" | "volunteer";
  familyOnly?: boolean;
  children: ReactNode;
}) {
  const { user } = useAuth();
  const isFamilyOnlyMember =
    user?.account_type === "member" && user.global_capabilities.length === 0;
  return user?.account_type === "member" &&
    (!capability || user.global_capabilities.includes(capability)) &&
    (!familyOnly || isFamilyOnlyMember) ? (
    children
  ) : (
    <Navigate to="/" replace />
  );
}

function LearningRoute({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const canAccessLearning =
    user?.account_type === "learner" ||
    (user?.account_type === "member" &&
      (user.global_capabilities.length === 0 ||
        user.global_capabilities.includes("volunteer")));
  return canAccessLearning ? children : <Navigate to="/" replace />;
}

function AdminRoute({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  return user?.global_capabilities.includes("admin") ? children : <Navigate to="/" replace />;
}

function App() {
  const { user, isLoading } = useAuth();

  if (isLoading) {
    return (
      <main
        className="grid min-h-screen place-items-center bg-canvas"
        aria-label="正在恢复会话"
      >
        <Spinner size="lg" />
      </main>
    );
  }

  if (!user) {
    return <LoginPage />;
  }

  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<DashboardPage />} />
        <Route
          path="learning"
          element={
            <LearningRoute>
              <LearningCenterPage />
            </LearningRoute>
          }
        />
        <Route path="profile" element={<ProfilePage />} />
        <Route
          path="admin/learning"
          element={
            <AdminRoute>
              <LearningGovernancePage />
            </AdminRoute>
          }
        />
        <Route
          path="family"
          element={
            <CaseRoleRoute familyOnly>
              <CaseWorkspacePage mode="family" />
            </CaseRoleRoute>
          }
        />
        <Route
          path="command"
          element={
            <CaseRoleRoute capability="commander">
              <CaseWorkspacePage mode="commander" />
            </CaseRoleRoute>
          }
        />
        <Route
          path="command/cases/:caseId"
          element={
            <CaseRoleRoute capability="commander">
              <CaseWorkspacePage mode="commander" />
            </CaseRoleRoute>
          }
        />
        <Route
          path="volunteer"
          element={
            <CaseRoleRoute capability="volunteer">
              <VolunteerWorkspacePage />
            </CaseRoleRoute>
          }
        />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export default App;
