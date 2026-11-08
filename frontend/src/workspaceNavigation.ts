import { BookOpen, HeartHandshake, LayoutDashboard, Navigation, RadioTower, ShieldCheck, UserRound } from "lucide-react";
import type { AuthUser } from "./api/auth";

export function workspaceAccess(user: AuthUser | null) {
  const member = user?.account_type === "member";
  return {
    family: member && user.global_capabilities.length === 0,
    commander: Boolean(member && user.global_capabilities.includes("commander")),
    volunteer: Boolean(member && user.global_capabilities.includes("volunteer")),
    admin: Boolean(user?.global_capabilities.includes("admin")),
    learning: Boolean(user?.account_type === "learner" || (member && (user.global_capabilities.length === 0 || user.global_capabilities.includes("volunteer")))),
  };
}

export function workspaceIdentity(user: AuthUser | null) {
  if (!user) return "";
  if (user.account_type === "learner") return "学习账号";
  const labels = { commander: "指挥人员", volunteer: "志愿者", admin: "管理员" };
  return user.global_capabilities.map((capability) => labels[capability]).join(" / ") || "家属 / 知情人";
}

export function workspaceNavigation(user: AuthUser | null) {
  const access = workspaceAccess(user);
  return [
    { to: "/", label: "总览", icon: LayoutDashboard, end: true },
    ...(access.family ? [{ to: "/family", label: "家属端", icon: HeartHandshake }] : []),
    ...(access.commander ? [{ to: "/command", label: "指挥端", icon: RadioTower }] : []),
    ...(access.volunteer ? [{ to: "/volunteer", label: "志愿者端", icon: Navigation }] : []),
    ...(access.admin ? [
      { to: "/admin/access-requests", label: "账号审核", icon: ShieldCheck },
      { to: "/admin/learning", label: "内容治理", icon: ShieldCheck },
    ] : []),
    ...(access.learning ? [{ to: "/learning", label: "学习中心", icon: BookOpen }] : []),
    { to: "/profile", label: "个人资料", icon: UserRound },
  ];
}
