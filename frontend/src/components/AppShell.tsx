import { Button, Spinner } from "@heroui/react";
import { LogOut, UserRound } from "lucide-react";
import type { CSSProperties } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { useAuth } from "../auth/useAuth";
import { workspaceIdentity, workspaceNavigation } from "../workspaceNavigation";
import { ServiceStatus } from "./ServiceStatus";
import "./AppShell.css";

export function AppShell() {
  const { user, logout, isLoggingOut } = useAuth();
  const { pathname } = useLocation();
  const navigation = workspaceNavigation(user);
  // Keep the full authorized navigation on desktop. Mobile gets overview,
  // at most two role destinations, and profile; the overview retains all entries.
  const mobilePaths = new Set(["/", ...navigation.slice(1, -1).slice(0, 2).map((item) => item.to), "/profile"]);
  const current = navigation.find((item) => item.to !== "/" && pathname.startsWith(item.to));

  return (
    <div className="app-shell">
      <a href="#workspace-content" className="app-shell-skip">跳转到工作区内容</a>
      <header className="app-shell-header">
        <Link to="/" className="app-shell-brand" aria-label="安归总览首页">
          <img src={brandMark} width={32} height={32} alt="" />
          <strong>安归</strong>
        </Link>
        <span className="app-shell-workspace">{current?.label ?? "总览工作台"}</span>
        <div className="app-shell-account">
          <Link to="/profile" className="app-shell-user" aria-label="个人账户">
            <span className="app-shell-avatar" aria-hidden="true"><UserRound size={18} /></span>
            <span className="app-shell-user-copy">
              <strong>{user?.display_name}</strong>
              <span>{workspaceIdentity(user)}</span>
            </span>
          </Link>
          <Button size="sm" variant="ghost" isIconOnly aria-label={isLoggingOut ? "正在退出登录" : "退出登录"} isDisabled={isLoggingOut} onPress={() => void logout()}>
            {isLoggingOut ? <Spinner size="sm" /> : <LogOut size={18} aria-hidden="true" />}
          </Button>
        </div>
      </header>
      <aside className="app-shell-sidebar">
        <p className="app-shell-nav-caption">工作空间</p>
        <nav className="app-shell-navigation" aria-label="主要导航" style={{ "--mobile-nav-columns": mobilePaths.size } as CSSProperties}>
          {navigation.map(({ to, label, icon: Icon, end }) => (
            <NavLink key={to} to={to} end={end} className={({ isActive }) => ["app-shell-nav-item", isActive ? "is-active" : "", mobilePaths.has(to) ? "" : "desktop-only"].join(" ")}>
              <Icon size={20} aria-hidden="true" /><span>{label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="app-shell-sidebar-footer">
          <ServiceStatus compact />
          <p>仅展示当前账号已获授权的工作区</p>
        </div>
      </aside>
      <main id="workspace-content" className="app-shell-content" tabIndex={-1}><Outlet /></main>
    </div>
  );
}
