import { Button, Input } from "@heroui/react";
import { MapPin, Radio, UsersRound } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  createCollaborationSpace,
  joinCollaborationSpace,
  leaveCollaborationSpace,
  listCollaborationSpaces,
  revokeSpaceLocationConsent,
  type CollaborationSpace,
} from "../api/collaborationSpaces";
import { ApiClientError } from "../api/client";
import { EmptyState, ErrorState, LoadingState } from "./ContentState";
import { StatusTag } from "./StatusTag";

export function CollaborationSpacePanel({ token, caseId, role }: { token: string | null; caseId: string; role: "commander" | "volunteer" | "family" }) {
  const [spaces, setSpaces] = useState<CollaborationSpace[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    if (!token || role === "family") return;
    setLoading(true); setError("");
    try { setSpaces(await listCollaborationSpaces(token, caseId)); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "无法加载协作空间"); }
    finally { setLoading(false); }
  }, [caseId, role, token]);
  useEffect(() => { void load(); }, [load]);
  if (role === "family") return null;
  const run = async (key: string, action: () => Promise<unknown>) => {
    if (!token) return; setBusy(key); setError("");
    try { await action(); await load(); }
    catch (cause) {
      const detail = cause instanceof ApiClientError ? cause.detail : undefined;
      setError(detail || (cause instanceof Error ? cause.message : "操作失败，请稍后重试"));
    } finally { setBusy(""); }
  };
  return <section className="border border-slate-200 bg-white p-4 sm:p-5" aria-labelledby="collaboration-space-title">
    <div className="flex flex-wrap items-start justify-between gap-3"><div><p className="m-0 text-xs font-semibold tracking-wide text-brand-700">行动协作</p><h2 id="collaboration-space-title" className="mt-1 text-lg font-semibold text-slate-950">协作空间</h2><p className="mt-1 text-sm text-slate-600">仅案件内指挥员和志愿者可进入。位置共享可随时撤回。</p></div><Radio aria-hidden="true" className="text-brand-700" /></div>
    {role === "commander" && <form className="mt-4 flex flex-col gap-2 sm:flex-row" onSubmit={(event) => { event.preventDefault(); const trimmed = name.trim(); if (trimmed) void run("create", async () => { await createCollaborationSpace(token!, caseId, trimmed); setName(""); }); }}><Input aria-label="协作空间名称" value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：东侧搜寻行动" maxLength={120} /><Button type="submit" variant="primary" isDisabled={!name.trim() || busy === "create"}>创建空间</Button></form>}
    {error && <ErrorState message={error} onRetry={() => void load()} />}
    {loading ? <LoadingState label="正在加载协作空间" /> : spaces.length === 0 ? <EmptyState title="暂无可进入的协作空间" description={role === "commander" ? "创建后可邀请本案件志愿者加入。" : "请等待指挥员创建行动空间。"} /> : <ul className="mt-4 grid list-none gap-3 p-0">{spaces.map((space) => <li className="border border-slate-200 p-3" key={space.id}><div className="flex flex-wrap items-center justify-between gap-2"><div><strong className="text-sm text-slate-950">{space.name}</strong><p className="mt-1 text-xs text-slate-600">版本 {space.current_version} · {space.member_status === "active" ? "已在空间中" : "尚未加入"}</p></div><StatusTag tone={space.status === "active" ? "confirmed" : "excluded"} label={space.status === "active" ? "行动中" : "已归档"} /></div><div className="mt-3 flex flex-wrap gap-2">{role === "volunteer" && space.member_status !== "active" && <Button size="sm" variant="primary" isDisabled={Boolean(busy)} onPress={() => void run(`join-${space.id}`, () => joinCollaborationSpace(token!, space.id, "location-consent-v1"))}><MapPin size={15} />同意并加入</Button>}{role === "volunteer" && space.member_status === "active" && <><Button size="sm" variant="secondary" isDisabled={Boolean(busy)} onPress={() => void run(`revoke-${space.id}`, () => revokeSpaceLocationConsent(token!, space.id))}>停止位置共享</Button><Button size="sm" variant="ghost" isDisabled={Boolean(busy)} onPress={() => void run(`leave-${space.id}`, () => leaveCollaborationSpace(token!, space.id))}><UsersRound size={15} />离开空间</Button></>}</div></li>)}</ul>}
  </section>;
}
