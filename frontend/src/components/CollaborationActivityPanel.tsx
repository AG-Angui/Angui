import { Button, TextArea } from "@heroui/react";
import { LocateFixed, MessageSquareText, Send } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import {
  getSpaceEvents,
  getSpaceSnapshot,
  listSpaceMessages,
  recordSpaceLocation,
  sendSpaceMessage,
  type SpaceEvent,
  type SpaceMessage,
  type SpaceSnapshot,
} from "../api/collaborationSpaces";
import { ErrorState, LoadingState } from "./ContentState";

const LOCATION_INTERVAL_MS = 15_000;

/** Snapshot-first polling is the local fallback when the realtime gateway is unavailable. */
export function CollaborationActivityPanel({ token, spaceId, canBroadcast }: { token: string; spaceId: string; canBroadcast: boolean }) {
  const [snapshot, setSnapshot] = useState<SpaceSnapshot | null>(null);
  const [messages, setMessages] = useState<SpaceMessage[]>([]);
  const [content, setContent] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const version = useRef(0);
  const locationAt = useRef(0);
  const load = useCallback(async () => {
    try {
      const [nextSnapshot, nextMessages] = await Promise.all([getSpaceSnapshot(token, spaceId), listSpaceMessages(token, spaceId)]);
      version.current = nextSnapshot.version; setSnapshot(nextSnapshot); setMessages(nextMessages); setError("");
    } catch (cause) { setError(cause instanceof Error ? cause.message : "无法同步协作空间"); }
  }, [spaceId, token]);
  useEffect(() => { void load(); }, [load]);
  useEffect(() => {
    const timer = window.setInterval(() => { void getSpaceEvents(token, spaceId, version.current).then((events) => {
      if (events.length) { version.current = Math.max(version.current, ...events.map((event) => event.version)); applyEvents(events, setMessages); void load(); }
    }).catch(() => undefined); }, 8_000);
    return () => window.clearInterval(timer);
  }, [load, spaceId, token]);
  const shareOnce = () => navigator.geolocation?.getCurrentPosition((position) => {
    if (Date.now() - locationAt.current < LOCATION_INTERVAL_MS) return;
    locationAt.current = Date.now();
    void recordSpaceLocation(token, spaceId, {
      latitude: position.coords.latitude, longitude: position.coords.longitude,
      accuracy_meters: position.coords.accuracy, captured_at: new Date(position.timestamp).toISOString(), operation_id: crypto.randomUUID(),
    }).catch((cause) => setError(cause instanceof Error ? cause.message : "位置上报失败"));
  }, () => setError("未获得定位权限；仍可继续使用任务和文字沟通。"), { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 });
  if (error && !snapshot) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!snapshot) return <LoadingState label="正在同步行动空间" />;
  return <section className="mt-3 border border-slate-200 bg-slate-50 p-3" aria-label="行动空间消息与状态">
    <div className="flex items-center justify-between gap-2"><strong className="text-sm text-slate-950">{snapshot.space.name} · 实时降级模式</strong><Button size="sm" variant="ghost" onPress={shareOnce}><LocateFixed size={15} />上报当前位置</Button></div>
    <p className="mt-2 text-xs text-slate-600">当前通过快照与增量事件补偿同步；地图网络不可用时仍保留成员、任务和消息文本。位置仅在你已同意且运营方配置保留策略后写入。</p>
    {error && <p className="mt-2 text-xs text-red-700" role="alert">{error}</p>}
    <div className="mt-3 max-h-48 overflow-auto border border-slate-200 bg-white p-2"><div className="mb-2 flex items-center gap-1 text-xs font-medium text-slate-600"><MessageSquareText size={14} />最近消息</div>{messages.length === 0 ? <p className="m-0 text-xs text-slate-500">暂无消息。</p> : messages.slice().reverse().map((message) => <p className="mb-2 text-sm text-slate-800" key={message.id}>{message.message_type === "broadcast" && <strong className="mr-1 text-red-800">[指挥广播]</strong>}{message.content}</p>)}</div>
    <form className="mt-2 flex gap-2" onSubmit={(event) => { event.preventDefault(); const text = content.trim(); if (!text) return; setBusy(true); void sendSpaceMessage(token, spaceId, text).then(() => { setContent(""); return load(); }).catch((cause) => setError(cause instanceof Error ? cause.message : "消息发送失败")).finally(() => setBusy(false)); }}><TextArea aria-label="发送协作消息" value={content} onChange={(event) => setContent(event.target.value)} maxLength={2000} placeholder="发送现场消息" /><Button type="submit" variant="primary" isDisabled={busy || !content.trim()}><Send size={15} />发送</Button>{canBroadcast && <Button type="button" variant="secondary" isDisabled={busy || !content.trim()} onPress={() => { const text = content.trim(); if (!text) return; setBusy(true); void sendSpaceMessage(token, spaceId, text, "broadcast").then(() => { setContent(""); return load(); }).catch((cause) => setError(cause instanceof Error ? cause.message : "广播发送失败")).finally(() => setBusy(false)); }}>广播</Button>}</form>
  </section>;
}

function applyEvents(events: SpaceEvent[], setMessages: Dispatch<SetStateAction<SpaceMessage[]>>) {
  for (const event of events) if (event.event_type === "message.sent") {
    const payload = event.payload;
    const { message_id: id, sender_id, content, sent_at } = payload;
    if (typeof id === "string" && typeof sender_id === "string" && typeof content === "string" && typeof sent_at === "string") {
      setMessages((current) => current.some((message) => message.id === id) ? current : [{ id, sender_id, message_type: payload.message_type === "broadcast" ? "broadcast" : "text", content, sent_at, recalled_at: null }, ...current]);
    }
  }
}
