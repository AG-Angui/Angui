import { Chip } from '@heroui/react'
import {
  HeartHandshake,
  LayoutDashboard,
  Navigation,
  RadioTower,
} from 'lucide-react'
import { NavLink, Outlet } from 'react-router-dom'
import brandMark from '../../../assets/brand/angui-mark.svg'
import { ServiceStatus } from './ServiceStatus'

const navigation = [
  { to: '/', label: '总览', icon: LayoutDashboard, end: true },
  { to: '/family', label: '家属端', icon: HeartHandshake },
  { to: '/command', label: '指挥端', icon: RadioTower },
  { to: '/volunteer', label: '志愿者端', icon: Navigation },
]

export function AppShell() {
  return (
    <div className="min-h-screen bg-canvas text-slate-700 lg:grid lg:grid-cols-[224px_minmax(0,1fr)]">
      <aside className="fixed inset-x-0 bottom-0 z-50 h-16 border-t border-slate-200 bg-white px-2 py-1.5 lg:sticky lg:top-0 lg:flex lg:h-screen lg:flex-col lg:border-r lg:border-t-0 lg:px-4 lg:py-5">
        <div className="hidden min-h-12 items-center gap-3 px-2 pb-5 lg:flex">
          <img
            src={brandMark}
            className="size-10 rounded-md"
            alt="安归"
          />
          <div className="min-w-0">
            <strong className="block text-lg leading-tight text-slate-950">安归</strong>
            <span className="mt-0.5 block text-xs text-slate-500">协同工作台</span>
          </div>
        </div>

        <nav className="grid h-full grid-cols-4 gap-1 lg:flex lg:h-auto lg:flex-col" aria-label="主要导航">
          {navigation.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                [
                  'flex min-h-12 items-center justify-center gap-1 rounded-md px-1 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 lg:min-h-10 lg:justify-start lg:gap-3 lg:px-3 lg:text-sm',
                  isActive
                    ? 'bg-brand-50 text-brand-700'
                    : 'text-slate-500 hover:bg-slate-100 hover:text-slate-950',
                ].join(' ')
              }
            >
              <Icon size={18} aria-hidden="true" />
              <span>{label}</span>
            </NavLink>
          ))}
        </nav>

        <div className="mt-auto hidden border-t border-slate-200 px-2 pt-4 lg:block">
          <ServiceStatus compact />
          <Chip size="sm" variant="soft" className="mt-2">
            <Chip.Label>开发环境</Chip.Label>
          </Chip>
        </div>
      </aside>

      <main className="min-w-0 pb-16 lg:pb-0">
        <Outlet />
      </main>
    </div>
  )
}
