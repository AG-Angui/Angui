import { Button, TextArea } from "@heroui/react";
import { LocateFixed, MessageSquareText, Mic, Send, Upload } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import {
  getSpaceEvents, getSpaceSnapshot, listLatestSpaceLocations, listSpaceMessages, listVoiceReports,
  recordSpaceLocation, sendSpaceMessage, uploadVoiceReport,
  type SpaceEvent, type SpaceLocation, type SpaceMessage, type SpaceSnapshot, type VoiceReport,
} from "../api/collaborationSpaces";
import { ErrorState, LoadingState } from "./ContentState";
import { CollaborationSpaceMap } from "./CollaborationSpaceMap";

const LOCATION_INTERVAL_MS = 15_000;
const formatMessageTime = (value: string) => new Intl.DateTimeFormat(undefined, { dateStyle: "short", timeStyle: "short" }).format(new Date(value));

export function CollaborationActivityPanel({ token, spaceId, canBroadcast }: { token: string; spaceId: string; canBroadcast: boolean }) {
  const [snapshot, setSnapshot] = useState<SpaceSnapshot | null>(null);
  const [messages, setMessages] = useState<SpaceMessage[]>([]);
  const [voiceReports, setVoiceReports] = useState<VoiceReport[]>([]);
  const [locations, setLocations] = useState<SpaceLocation[]>([]);
  const [content, setContent] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [isSharingLocation, setIsSharingLocation] = useState(false);
  const version = useRef(0);
  const locationAt = useRef(0);
  const watchId = useRef<number | null>(null);

  const load = useCallback(async () => {
    try {
      const [nextSnapshot, nextMessages, nextVoiceReports, nextLocations] = await Promise.all([
        getSpaceSnapshot(token, spaceId), listSpaceMessages(token, spaceId), listVoiceReports(token, spaceId), listLatestSpaceLocations(token, spaceId),
      ]);
      version.current = nextSnapshot.version;
      setSnapshot(nextSnapshot); setMessages(nextMessages); setVoiceReports(nextVoiceReports); setLocations(nextLocations); setError("");
    } catch (cause) { setError(cause instanceof Error ? cause.message : "无法同步协作空间"); }
  }, [spaceId, token]);
  useEffect(() => { void load(); }, [load]);
  useEffect(() => {
    const timer = window.setInterval(() => { void getSpaceEvents(token, spaceId, version.current).then((events) => {
      if (events.length) { version.current = Math.max(version.current, ...events.map((event) => event.version)); applyEvents(events, setMessages); void load(); }
    }).catch(() => undefined); }, 8_000);
    return () => window.clearInterval(timer);
  }, [load, spaceId, token]);
  useEffect(() => () => { if (watchId.current !== null) navigator.geolocation?.clearWatch(watchId.current); }, []);

  const recordPosition = (position: GeolocationPosition) => {
    if (Date.now() - locationAt.current < LOCATION_INTERVAL_MS) return;
    locationAt.current = Date.now();
    void recordSpaceLocation(token, spaceId, {
      latitude: position.coords.latitude, longitude: position.coords.longitude, accuracy_meters: position.coords.accuracy,
      captured_at: new Date(position.timestamp).toISOString(), operation_id: crypto.randomUUID(),
    }).then((location) => setLocations((current) => [location, ...current.filter((item) => item.user_id !== location.user_id)])).catch((cause) => setError(cause instanceof Error ? cause.message : "位置上报失败"));
  };
  const shareOnce = () => navigator.geolocation?.getCurrentPosition(recordPosition, () => setError("未获取定位权限；仍可继续使用任务和文字沟通。"), { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 });
  const startSharing = () => {
    if (!navigator.geolocation) { setError("当前设备不支持定位"); return; }
    setError("");
    navigator.geolocation.getCurrentPosition((position) => {
      recordPosition(position);
      const id = navigator.geolocation!.watchPosition(recordPosition, () => setError("位置共享暂时失去定位权限"), { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 });
      watchId.current = id; setIsSharingLocation(true);
    }, () => setError("未获取定位权限；请允许后再开始共享。"), { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 });
  };
  const stopSharing = () => { if (watchId.current !== null) navigator.geolocation?.clearWatch(watchId.current); watchId.current = null; setIsSharingLocation(false); };

  if (error && !snapshot) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!snapshot) return <LoadingState label="正在同步协作空间" />;
  return <section className="mt-3 border border-slate-200 bg-slate-50 p-3" aria-label="协作空间消息与状态">
    <div className="flex items-center justify-between gap-2"><strong className="text-sm text-slate-950">{snapshot.space.name} · 实时协作</strong><div className="flex flex-wrap gap-2"><Button size="sm" variant="secondary" onPress={isSharingLocation ? stopSharing : startSharing}><LocateFixed size={15} />{isSharingLocation ? "停止位置共享" : "开始位置共享"}</Button><Button size="sm" variant="ghost" onPress={shareOnce}><LocateFixed size={15} />上报当前位置</Button></div></div>
    <p className="mt-2 text-xs text-slate-600">位置共享可随时停止；地图仅展示当前成员的最新位置。</p>
    {error && <p className="mt-2 text-xs text-red-700" role="alert">{error}</p>}
    <CollaborationSpaceMap members={snapshot.members} locations={locations} />
    <div className="mt-3 max-h-48 overflow-auto border border-slate-200 bg-white p-2"><div className="mb-2 flex items-center gap-1 text-xs font-medium text-slate-600"><MessageSquareText size={14} />最近消息</div>{messages.length === 0 ? <p className="m-0 text-xs text-slate-500">暂无消息。</p> : messages.slice().reverse().map((message) => <div className="mb-2 text-sm text-slate-800" key={message.id}>{message.message_type === "broadcast" && <strong className="mr-1 text-red-800">[指挥广播]</strong>}{message.content}<div className="mt-0.5 text-xs text-slate-500">{message.sender_display_name || "未知用户"} · {formatMessageTime(message.sent_at)}</div></div>)}</div>
    <form className="mt-2 flex gap-2" onSubmit={(event) => { event.preventDefault(); const text = content.trim(); if (!text) return; setBusy(true); void sendSpaceMessage(token, spaceId, text).then(() => { setContent(""); return load(); }).catch((cause) => setError(cause instanceof Error ? cause.message : "消息发送失败")).finally(() => setBusy(false)); }}><TextArea aria-label="发送协作消息" value={content} onChange={(event) => setContent(event.target.value)} maxLength={2000} placeholder="发送现场消息" /><Button type="submit" variant="primary" isDisabled={busy || !content.trim()}><Send size={15} />发送</Button>{canBroadcast && <Button type="button" variant="secondary" isDisabled={busy || !content.trim()} onPress={() => { const text = content.trim(); if (!text) return; setBusy(true); void sendSpaceMessage(token, spaceId, text, "broadcast").then(() => { setContent(""); return load(); }).catch((cause) => setError(cause instanceof Error ? cause.message : "广播发送失败")).finally(() => setBusy(false)); }}>广播</Button>}</form>
    <div className="mt-3 border-t border-slate-200 pt-3"><div className="flex items-center gap-1 text-xs font-medium text-slate-700"><Mic size={14} />Voice reports</div><label className="mt-2 flex items-center gap-2 text-xs text-slate-600"><input aria-label="Upload a voice report" accept="audio/mpeg,audio/ogg,audio/wav,audio/webm" className="block max-w-full text-xs" disabled={busy} type="file" onChange={(event) => { const file = event.currentTarget.files?.[0]; event.currentTarget.value = ""; if (!file) return; setBusy(true); void uploadVoiceReport(token, spaceId, file).then(() => load()).catch((cause) => setError(cause instanceof Error ? cause.message : "Voice report upload failed")).finally(() => setBusy(false)); }} /><Upload size={14} /></label>{voiceReports.length > 0 && <div className="mt-2 max-h-32 overflow-auto text-xs text-slate-700">{voiceReports.map((report) => <div className="border-b border-slate-100 py-1" key={report.id}><span className="font-medium">{report.status}</span><span className="ml-2">{Math.ceil(report.byte_size / 1024)} KB</span>{report.failed_reason && <span className="ml-2 text-amber-800">{report.failed_reason}</span>}{canBroadcast && report.transcript && <p className="mt-1 whitespace-pre-wrap text-slate-800">{report.transcript.content}</p>}</div>)}</div>}</div>
  </section>;
}

function applyEvents(events: SpaceEvent[], setMessages: Dispatch<SetStateAction<SpaceMessage[]>>) {
  for (const event of events) if (event.event_type === "message.sent") {
    const payload = event.payload;
    const { message_id: id, sender_id, content, sent_at, sender_display_name } = payload;
    if (typeof id === "string" && typeof sender_id === "string" && typeof content === "string" && typeof sent_at === "string") setMessages((current) => current.some((message) => message.id === id) ? current : [{ id, sender_id, sender_display_name: typeof sender_display_name === "string" ? sender_display_name : "未知用户", message_type: payload.message_type === "broadcast" ? "broadcast" : "text", content, sent_at, recalled_at: null }, ...current]);
  }
}
