import { Button, Chip, Spinner } from "@heroui/react";
import {
  BookOpen,
  ShieldCheck,
  HeartHandshake,
  LayoutDashboard,
  LogOut,
  Navigation,
  RadioTower,
  UserRound,
} from "lucide-react";
import { NavLink, Outlet } from "react-router";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { useAuth } from "../auth/useAuth";
import type { AccountType, GlobalCapability } from "../api/auth";
import { ServiceStatus } from "./ServiceStatus";

interface NavigationItem {
  to: string;
  label: string;
  icon: typeof LayoutDashboard;
  end?: boolean;
  capability?: "commander" | "volunteer";
  familyOnly?: boolean;
  learningAudience?: boolean;
  adminOnly?: boolean;
}

const navigation: NavigationItem[] = [
  { to: "/", label: "总览", icon: LayoutDashboard, end: true },
  {
    to: "/learning",
    label: "学习中心",
    icon: BookOpen,
    learningAudience: true,
  },
  {
    to: "/admin/learning",
    label: "内容治理",
    icon: ShieldCheck,
    adminOnly: true,
  },
  { to: "/family", label: "家属端", icon: HeartHandshake, familyOnly: true },
  {
    to: "/command",
    label: "指挥端",
    icon: RadioTower,
    capability: "commander",
  },
  {
    to: "/volunteer",
    label: "志愿者端",
    icon: Navigation,
    capability: "volunteer",
  },
  { to: "/profile", label: "个人资料", icon: UserRound },
];

const roleLabels: Record<GlobalCapability, string> = {
  commander: "指挥",
  volunteer: "志愿者",
  admin: "管理员",
};

function identityLabel(
  accountType: AccountType,
  capabilities: GlobalCapability[],
) {
  const labels = capabilities.map((capability) => roleLabels[capability]);
  return labels.length > 0
    ? `${accountType} / ${labels.join(" / ")}`
    : accountType;
}

export function AppShell() {
  const { user, logout, isLoggingOut } = useAuth();
  const visibleNavigation = navigation.filter(
    (item) =>
      user &&
      (item.to === "/" ||
        item.to === "/profile" ||
        (item.learningAudience &&
          (user.account_type === "learner" ||
            (user.account_type === "member" &&
              (user.global_capabilities.length === 0 ||
                user.global_capabilities.includes("volunteer"))))) ||
        (item.adminOnly && user.global_capabilities.includes("admin")) ||
        (!item.learningAudience &&
          !item.adminOnly &&
          user.account_type === "member" &&
          (!item.capability ||
            user.global_capabilities.includes(item.capability)) &&
          (!item.familyOnly || user.global_capabilities.length === 0))),
  );

  return (
    <div className="min-h-screen bg-canvas text-slate-700 lg:grid lg:grid-cols-[224px_minmax(0,1fr)]">
      <aside className="fixed inset-x-0 bottom-0 z-50 h-16 border-t border-slate-200 bg-white px-2 py-1.5 lg:sticky lg:top-0 lg:flex lg:h-screen lg:flex-col lg:border-r lg:border-t-0 lg:px-4 lg:py-5">
        <div className="hidden min-h-12 items-center gap-3 px-2 pb-5 lg:flex">
          <img src={brandMark} className="size-10 rounded-md" alt="安归" />
          <div className="min-w-0">
            <strong className="block text-lg leading-tight text-slate-950">
              安归
            </strong>
            <span className="mt-0.5 block text-xs text-slate-500">
              协同工作台
            </span>
          </div>
        </div>

        <nav
          className="grid h-full gap-1 lg:flex lg:h-auto lg:flex-col"
          style={{
            gridTemplateColumns: `repeat(${Math.max(visibleNavigation.length, 1)}, minmax(0, 1fr))`,
          }}
          aria-label="主要导航"
        >
          {visibleNavigation.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                [
                  "flex min-h-12 items-center justify-center gap-1 rounded-md px-1 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 lg:min-h-10 lg:justify-start lg:gap-3 lg:px-3 lg:text-sm",
                  isActive
                    ? "bg-brand-50 text-brand-700"
                    : "text-slate-500 hover:bg-slate-100 hover:text-slate-950",
                ].join(" ")
              }
            >
              <Icon size={18} aria-hidden="true" />
              <span>{label}</span>
            </NavLink>
          ))}
        </nav>

        <div className="mt-auto hidden border-t border-slate-200 px-2 pt-4 lg:block">
          {user && (
            <div className="mb-3 min-w-0">
              <div className="flex items-center gap-2">
                <strong className="truncate text-sm text-slate-950">
                  {user.display_name}
                </strong>
                <Chip size="sm" variant="soft">
                  <Chip.Label>
                    {identityLabel(user.account_type, user.global_capabilities)}
                  </Chip.Label>
                </Chip>
              </div>
              <span className="mt-1 block truncate text-xs text-slate-500">
                {user.email}
              </span>
            </div>
          )}
          <ServiceStatus compact />
          <Button
            className="mt-3"
            size="sm"
            variant="ghost"
            fullWidth
            isDisabled={isLoggingOut}
            onPress={() => void logout()}
          >
            {isLoggingOut ? (
              <Spinner size="sm" aria-label="正在退出登录" />
            ) : (
              <LogOut size={16} aria-hidden="true" />
            )}
            {isLoggingOut ? "正在退出" : "退出登录"}
          </Button>
        </div>
      </aside>

      <main className="min-w-0 pb-16 lg:pb-0">
        <div className="flex min-h-14 items-center justify-between border-b border-slate-200 bg-white px-4 lg:hidden">
          <div className="min-w-0">
            <strong className="block truncate text-sm text-slate-950">
              {user?.display_name}
            </strong>
            <span className="block truncate text-xs text-slate-500">
              {user
                ? identityLabel(user.account_type, user.global_capabilities)
                : ""}
            </span>
          </div>
          <Button
            size="sm"
            variant="ghost"
            isIconOnly
            aria-label={isLoggingOut ? "正在退出登录" : "退出登录"}
            isDisabled={isLoggingOut}
            onPress={() => void logout()}
          >
            {isLoggingOut ? (
              <Spinner size="sm" />
            ) : (
              <LogOut size={17} aria-hidden="true" />
            )}
          </Button>
        </div>
        <Outlet />
      </main>
    </div>
  );
}
