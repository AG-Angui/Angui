import {
  HeartHandshake,
  LayoutDashboard,
  MapPinned,
  Navigation,
  RadioTower,
} from 'lucide-react'
import { NavLink, Outlet } from 'react-router-dom'
import { ServiceStatus } from './ServiceStatus'

const navigation = [
  { to: '/', label: '总览', icon: LayoutDashboard, end: true },
  { to: '/family', label: '家属端', icon: HeartHandshake },
  { to: '/command', label: '指挥端', icon: RadioTower },
  { to: '/volunteer', label: '志愿者端', icon: Navigation },
]

export function AppShell() {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            <MapPinned size={21} strokeWidth={2.2} />
          </span>
          <div>
            <strong>安归</strong>
            <span>协同工作台</span>
          </div>
        </div>

        <nav className="primary-nav" aria-label="主要导航">
          {navigation.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                `nav-link${isActive ? ' nav-link-active' : ''}`
              }
            >
              <Icon size={18} aria-hidden="true" />
              <span>{label}</span>
            </NavLink>
          ))}
        </nav>

        <div className="sidebar-footer">
          <ServiceStatus compact />
          <span className="environment-label">开发环境</span>
        </div>
      </aside>

      <main className="main-content">
        <Outlet />
      </main>
    </div>
  )
}
