import { Chip } from "@heroui/react";
import {
  AlertTriangle,
  CheckCircle2,
  CircleHelp,
  Clock3,
  XCircle,
} from "lucide-react";

export type StatusTone = "confirmed" | "pending" | "excluded" | "danger" | "neutral";

const copy: Record<StatusTone, { label: string; icon: typeof CheckCircle2; className: string }> = {
  confirmed: { label: "已确认", icon: CheckCircle2, className: "bg-emerald-50 text-emerald-800" },
  pending: { label: "待核实", icon: Clock3, className: "bg-amber-50 text-amber-900" },
  excluded: { label: "已排除", icon: XCircle, className: "bg-slate-100 text-slate-700" },
  danger: { label: "危险", icon: AlertTriangle, className: "bg-red-50 text-red-800" },
  neutral: { label: "状态未知", icon: CircleHelp, className: "bg-slate-100 text-slate-700" },
};

export function statusTone(value: string): StatusTone {
  if (["confirmed", "resolved", "completed", "active"].includes(value)) return "confirmed";
  if (["pending_review", "needs_verification", "assigned", "in_progress"].includes(value)) return "pending";
  if (["rejected", "excluded", "closed", "expired"].includes(value)) return "excluded";
  if (["danger", "high"].includes(value)) return "danger";
  return "neutral";
}

export function StatusTag({ tone, label }: { tone: StatusTone; label?: string }) {
  const item = copy[tone];
  const Icon = item.icon;
  return (
    <Chip className={item.className} size="sm" variant="soft">
      <Chip.Label className="inline-flex items-center gap-1">
        <Icon aria-hidden="true" size={14} />
        {label ?? item.label}
      </Chip.Label>
    </Chip>
  );
}
