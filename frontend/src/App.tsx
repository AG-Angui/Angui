import { Navigate, Route, Routes } from 'react-router-dom'
import { AppShell } from './components/AppShell'
import { DashboardPage } from './pages/DashboardPage'
import { WorkspacePage } from './pages/WorkspacePage'

function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<DashboardPage />} />
        <Route
          path="family"
          element={
            <WorkspacePage
              context="家属端"
              title="走失求助"
              emptyTitle="当前没有进行中的求助"
              emptyDescription="已提交并获得授权的案件会显示在这里。"
            />
          }
        />
        <Route
          path="command"
          element={
            <WorkspacePage
              context="指挥端"
              title="案件指挥"
              emptyTitle="当前没有活动案件"
              emptyDescription="案件、待审核线索和任务态势会显示在这里。"
            />
          }
        />
        <Route
          path="volunteer"
          element={
            <WorkspacePage
              context="志愿者端"
              title="我的任务"
              emptyTitle="当前没有待执行任务"
              emptyDescription="经指挥人员确认并分配给你的任务会显示在这里。"
            />
          }
        />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}

export default App
