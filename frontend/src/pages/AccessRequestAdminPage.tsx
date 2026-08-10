import { Button, Card, Spinner } from "@heroui/react";
import { Check, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { listAccessRequests, reviewAccessRequest, type AdminAccessRequest } from "../api/accessRequests";
import { useAuth } from "../auth/useAuth";

export function AccessRequestAdminPage() {
  const { token } = useAuth();
  const [items, setItems] = useState<AdminAccessRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const load = useCallback(async () => { if (!token) return; setLoading(true); try { setItems(await listAccessRequests(token)); } catch (cause) { setError(cause instanceof Error ? cause.message : "暂时无法加载申请"); } finally { setLoading(false); } }, [token]);
  useEffect(() => { void load(); }, [load]);
  async function review(item: AdminAccessRequest, action: "approve" | "reject") { if (!token) return; const reason = action === "reject" ? window.prompt("请输入拒绝原因") : undefined; if (action === "reject" && !reason) return; await reviewAccessRequest(token, item.id, { action, role: item.requested_role, ...(reason ? { reason } : {}) }); await load(); }
  return <section className="mx-auto max-w-6xl p-4 lg:p-8"><div className="mb-6"><h1 className="text-2xl font-semibold text-slate-950">账号访问审核</h1><p className="mt-1 text-sm text-slate-500">只审核已完成邮箱验证的访问申请，批准后发送设置密码邮件。</p></div>{error && <p className="mb-4 rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">{error}</p>}{loading ? <div className="grid min-h-48 place-items-center"><Spinner /></div> : <div className="grid gap-4 md:grid-cols-2">{items.filter(item => item.status === "pending_review").map(item => <Card key={item.id} className="border border-slate-200 shadow-none"><Card.Content className="space-y-3 p-5"><div className="flex items-start justify-between gap-4"><div><h2 className="font-semibold text-slate-950">{item.display_name}</h2><p className="text-sm text-slate-500">{item.email}</p></div><span className="rounded-full bg-amber-50 px-2 py-1 text-xs text-amber-800">待审核</span></div><dl className="grid grid-cols-2 gap-3 text-sm"><div><dt className="text-slate-500">申请身份</dt><dd className="mt-1 font-medium">{item.requested_role}</dd></div><div><dt className="text-slate-500">邮箱状态</dt><dd className="mt-1 font-medium text-emerald-700">已验证</dd></div></dl><div className="flex gap-2"><Button variant="primary" onPress={() => void review(item, "approve")}><Check size={16} />批准并发送设密邮件</Button><Button variant="secondary" onPress={() => void review(item, "reject")}><X size={16} />拒绝</Button></div></Card.Content></Card>)}</div>}</section>;
}
