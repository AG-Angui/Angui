import { Button, TextArea } from "@heroui/react";
import {
  LocateFixed,
  MessageSquareText,
  Mic,
  Send,
  Upload,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import {
  acknowledgeSpaceMessage,
  getSpaceEvents,
  getSpaceSnapshot,
  listLatestSpaceLocations,
  listSpaceMessages,
  listVoiceReports,
  listVoiceCandidates,
  retryVoiceCandidate,
  confirmVoiceDiscoverer,
  returnVoiceCandidate,
  resubmitVoiceCandidate,
  mergeVoiceCandidate,
  recordSpaceLocation,
  sendSpaceMessage,
  uploadVoiceReport,
  type SpaceEvent,
  type SpaceLocation,
  type SpaceMessage,
  type SpaceSnapshot,
  type VoiceReport,
  type VoiceCandidate,
} from "../api/collaborationSpaces";
import { ErrorState, LoadingState } from "./ContentState";
import { CollaborationSpaceMap } from "./CollaborationSpaceMap";
import { useCollaborationSocket } from "../hooks/useCollaborationSocket";
import { VoiceIntercomPanel } from "./VoiceIntercomPanel";
import type { ClueDraftCandidate } from "../api/cases";

const LOCATION_INTERVAL_MS = 15_000;
const formatMessageTime = (value: string) =>
  new Intl.DateTimeFormat(undefined, {
    dateStyle: "short",
    timeStyle: "short",
  }).format(new Date(value));

export function CollaborationActivityPanel({
  token,
  spaceId,
  canBroadcast,
}: {
  token: string;
  spaceId: string;
  canBroadcast: boolean;
}) {
  const [snapshot, setSnapshot] = useState<SpaceSnapshot | null>(null);
  const [messages, setMessages] = useState<SpaceMessage[]>([]);
  const [voiceReports, setVoiceReports] = useState<VoiceReport[]>([]);
  const [voiceCandidates, setVoiceCandidates] = useState<VoiceCandidate[]>([]);
  const [locations, setLocations] = useState<SpaceLocation[]>([]);
  const [content, setContent] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [isSharingLocation, setIsSharingLocation] = useState(false);
  const version = useRef(0);
  const locationAt = useRef(0);
  const watchId = useRef<number | null>(null);
  const acknowledgedMessages = useRef(new Set<string>());

  const acknowledge = useCallback(
    (messageId: string) => {
      if (acknowledgedMessages.current.has(messageId)) return;
      acknowledgedMessages.current.add(messageId);
      void acknowledgeSpaceMessage(token, spaceId, messageId).catch(() => {
        acknowledgedMessages.current.delete(messageId);
      });
    },
    [spaceId, token],
  );

  const load = useCallback(async () => {
    try {
      const [
        nextSnapshot,
        nextMessages,
        nextVoiceReports,
        nextLocations,
        nextCandidates,
      ] = await Promise.all([
        getSpaceSnapshot(token, spaceId),
        listSpaceMessages(token, spaceId),
        listVoiceReports(token, spaceId),
        listLatestSpaceLocations(token, spaceId),
        listVoiceCandidates(token, spaceId),
      ]);
      version.current = nextSnapshot.version;
      setSnapshot(nextSnapshot);
      setMessages(nextMessages);
      setVoiceReports(nextVoiceReports);
      setLocations(nextLocations);
      setVoiceCandidates(nextCandidates);
      setError("");
      for (const message of nextMessages) acknowledge(message.id);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法同步协作空间");
    }
  }, [acknowledge, spaceId, token]);
  useEffect(() => {
    void load();
  }, [load]);
  useCollaborationSocket(token, spaceId, version.current, (events) => {
    if (events.length) {
      version.current = Math.max(
        version.current,
        ...events.map((event) => event.version),
      );
      applyEvents(events, setMessages, acknowledge);
    }
  });
  useEffect(() => {
    const timer = window.setInterval(() => {
      void getSpaceEvents(token, spaceId, version.current)
        .then((events) => {
          if (events.length) {
            version.current = Math.max(
              version.current,
              ...events.map((event) => event.version),
            );
            applyEvents(events, setMessages, acknowledge);
            void load();
          }
        })
        .catch(() => undefined);
    }, 8_000);
    return () => window.clearInterval(timer);
  }, [acknowledge, load, spaceId, token]);
  useEffect(() => {
    const timer = window.setInterval(() => {
      void Promise.all([
        listVoiceReports(token, spaceId),
        listVoiceCandidates(token, spaceId),
      ])
        .then(([reports, candidates]) => {
          setVoiceReports(reports);
          setVoiceCandidates(candidates);
        })
        .catch(() => undefined);
    }, 10_000);
    return () => window.clearInterval(timer);
  }, [spaceId, token]);
  useEffect(
    () => () => {
      if (watchId.current !== null)
        navigator.geolocation?.clearWatch(watchId.current);
    },
    [],
  );

  const recordPosition = (position: GeolocationPosition, force = false) => {
    if (!force && Date.now() - locationAt.current < LOCATION_INTERVAL_MS)
      return;
    locationAt.current = Date.now();
    void recordSpaceLocation(token, spaceId, {
      latitude: position.coords.latitude,
      longitude: position.coords.longitude,
      accuracy_meters: position.coords.accuracy,
      captured_at: new Date(position.timestamp).toISOString(),
      operation_id: crypto.randomUUID(),
    })
      .then((location) =>
        setLocations((current) => [
          location,
          ...current.filter((item) => item.user_id !== location.user_id),
        ]),
      )
      .catch((cause) =>
        setError(cause instanceof Error ? cause.message : "位置上报失败"),
      );
  };
  const shareOnce = () =>
    navigator.geolocation?.getCurrentPosition(
      recordPosition,
      () => setError("未获取定位权限；仍可继续使用任务和文字沟通。"),
      { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 },
    );
  const startSharing = () => {
    if (!navigator.geolocation) {
      setError("当前设备不支持定位");
      return;
    }
    setError("");
    navigator.geolocation.getCurrentPosition(
      (position) => {
        recordPosition(position, true);
        const id = navigator.geolocation!.watchPosition(
          recordPosition,
          () => setError("位置共享暂时失去定位权限"),
          { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 },
        );
        watchId.current = id;
        setIsSharingLocation(true);
      },
      () => setError("未获取定位权限；请允许后再开始共享。"),
      { enableHighAccuracy: false, maximumAge: 10_000, timeout: 8_000 },
    );
  };
  const stopSharing = () => {
    if (watchId.current !== null)
      navigator.geolocation?.clearWatch(watchId.current);
    watchId.current = null;
    setIsSharingLocation(false);
  };
  if (error && !snapshot)
    return <ErrorState message={error} onRetry={() => void load()} />;
  if (!snapshot) return <LoadingState label="正在同步协作空间" />;
  return (
    <section
      className="mt-3 border border-slate-200 bg-slate-50 p-3"
      aria-label="协作空间消息与状态"
    >
      <div className="flex items-center justify-between gap-2">
        <strong className="text-sm text-slate-950">
          {snapshot.space.name} · 实时协作
        </strong>
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="secondary"
            onPress={isSharingLocation ? stopSharing : startSharing}
          >
            <LocateFixed size={15} />
            {isSharingLocation ? "停止位置共享" : "开始位置共享"}
          </Button>
          <Button size="sm" variant="ghost" onPress={shareOnce}>
            <LocateFixed size={15} />
            上报当前位置
          </Button>
        </div>
      </div>
      <p className="mt-2 text-xs text-slate-600">
        位置共享可随时停止；地图仅展示当前成员的最新位置。
      </p>
      {error && (
        <p className="mt-2 text-xs text-red-700" role="alert">
          {error}
        </p>
      )}
      <CollaborationSpaceMap members={snapshot.members} locations={locations} />
      <div className="mt-3 max-h-48 overflow-auto border border-slate-200 bg-white p-2">
        <div className="mb-2 flex items-center gap-1 text-xs font-medium text-slate-600">
          <MessageSquareText size={14} />
          最近消息
        </div>
        {messages.length === 0 ? (
          <p className="m-0 text-xs text-slate-500">暂无消息。</p>
        ) : (
          messages
            .slice()
            .reverse()
            .map((message) => (
              <div className="mb-2 text-sm text-slate-800" key={message.id}>
                {message.message_type === "broadcast" && (
                  <strong className="mr-1 text-red-800">[指挥广播]</strong>
                )}
                {message.content}
                <div className="mt-0.5 text-xs text-slate-500">
                  {message.sender_display_name || "未知用户"} ·{" "}
                  {formatMessageTime(message.sent_at)}
                </div>
              </div>
            ))
        )}
      </div>
      <form
        className="mt-2 flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          const text = content.trim();
          if (!text) return;
          setBusy(true);
          void sendSpaceMessage(token, spaceId, text)
            .then(() => {
              setContent("");
              return load();
            })
            .catch((cause) =>
              setError(cause instanceof Error ? cause.message : "消息发送失败"),
            )
            .finally(() => setBusy(false));
        }}
      >
        <TextArea
          aria-label="发送协作消息"
          value={content}
          onChange={(event) => setContent(event.target.value)}
          maxLength={2000}
          placeholder="发送现场消息"
        />
        <Button
          type="submit"
          variant="primary"
          isDisabled={busy || !content.trim()}
        >
          <Send size={15} />
          发送
        </Button>
        {canBroadcast && (
          <Button
            type="button"
            variant="secondary"
            isDisabled={busy || !content.trim()}
            onPress={() => {
              const text = content.trim();
              if (!text) return;
              setBusy(true);
              void sendSpaceMessage(token, spaceId, text, "broadcast")
                .then(() => {
                  setContent("");
                  return load();
                })
                .catch((cause) =>
                  setError(
                    cause instanceof Error ? cause.message : "广播发送失败",
                  ),
                )
                .finally(() => setBusy(false));
            }}
          >
            广播
          </Button>
        )}
      </form>
      <div className="mt-3 border-t border-slate-200 pt-3">
        <div className="flex items-center gap-1 text-xs font-medium text-slate-700">
          <Mic size={14} />
          语音回传
        </div>
        <label className="mt-2 flex items-center gap-2 text-xs text-slate-600">
          <input
            aria-label="上传语音回传"
            accept="audio/mpeg,audio/ogg,audio/wav,audio/webm"
            className="block max-w-full text-xs"
            disabled={busy}
            type="file"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = "";
              if (!file) return;
              setBusy(true);
              void uploadVoiceReport(token, spaceId, file)
                .then(() => load())
                .catch((cause) =>
                  setError(
                    cause instanceof Error ? cause.message : "语音回传上传失败",
                  ),
                )
                .finally(() => setBusy(false));
            }}
          />
          <Upload size={14} />
        </label>
        {voiceReports.length > 0 && (
          <div className="mt-2 max-h-32 overflow-auto text-xs text-slate-700">
            {voiceReports.map((report) => (
              <div className="border-b border-slate-100 py-1" key={report.id}>
                <span className="font-medium">{report.status}</span>
                <span className="ml-2">
                  {Math.ceil(report.byte_size / 1024)} KB
                </span>
                {report.failed_reason && (
                  <span className="ml-2 text-amber-800">
                    {report.failed_reason}
                  </span>
                )}
                {canBroadcast && report.transcript && (
                  <p className="mt-1 whitespace-pre-wrap text-slate-800">
                    {report.transcript.content}
                  </p>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
      <VoiceIntercomPanel
        token={token}
        spaceId={spaceId}
        onUploaded={() => void load()}
      />
      {voiceCandidates.length > 0 && (
        <div
          className="mt-3 border-t border-slate-200 pt-3 text-xs"
          aria-label="语音线索处理状态"
        >
          <strong>语音线索候选</strong>
          {voiceCandidates.map((candidate) => (
            <div
              className="mt-2 rounded border border-slate-200 bg-white p-2"
              key={candidate.id}
            >
              <span>
                {candidate.source_type === "intercom_recording"
                  ? "对讲录音"
                  : "语音回传"}{" "}
                · {candidate.status}
              </span>
              {candidate.failure_reason && (
                <span className="ml-2 text-red-700">
                  {candidate.failure_reason}
                </span>
              )}
              {candidate.clue_draft_id && (
                <span className="ml-2">
                  待审核草稿：{candidate.clue_draft_id}
                </span>
              )}
              {candidate.returned_for_revision && (
                <p className="mt-1 text-amber-800">
                  已退回修改：{candidate.review_note}
                </p>
              )}
              {canBroadcast && candidate.transcript && (
                <p className="mt-1 whitespace-pre-wrap">
                  {candidate.transcript}
                </p>
              )}
              {candidate.status === "failed" && candidate.retry_count < 5 && (
                <button
                  className="ml-2 underline"
                  type="button"
                  onClick={() =>
                    void retryVoiceCandidate(token, spaceId, candidate.id)
                      .then(load)
                      .catch((cause) =>
                        setError(
                          cause instanceof Error ? cause.message : "重试失败",
                        ),
                      )
                  }
                >
                  重试
                </button>
              )}
              {candidate.status === "pending_review" &&
                candidate.can_confirm_discoverer && (
                  <div className="mt-1 flex items-center gap-2">
                    <label>
                      发现人
                      <select
                        className="ml-1 border p-1"
                        defaultValue={
                          candidate.discoverer_user_id ??
                          candidate.submitted_by_user_id
                        }
                        id={`discoverer-${candidate.id}`}
                      >
                        {snapshot.members
                          .filter((member) => member.status === "active")
                          .map((member) => (
                            <option key={member.user_id} value={member.user_id}>
                              {member.display_name}
                            </option>
                          ))}
                      </select>
                    </label>
                    <button
                      className="underline"
                      type="button"
                      onClick={() => {
                        const select = document.getElementById(
                          `discoverer-${candidate.id}`,
                        ) as HTMLSelectElement | null;
                        if (select)
                          void confirmVoiceDiscoverer(
                            token,
                            spaceId,
                            candidate.id,
                            select.value,
                          )
                            .then(load)
                            .catch((cause) =>
                              setError(
                                cause instanceof Error
                                  ? cause.message
                                  : "发现人确认失败",
                              ),
                            );
                      }}
                    >
                      确认发现人
                    </button>
                  </div>
                )}
              {candidate.status === "pending_review" && (
                <VoiceCandidateActions
                  candidate={candidate}
                  commander={canBroadcast}
                  token={token}
                  spaceId={spaceId}
                  onChanged={load}
                  onError={setError}
                />
              )}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function VoiceCandidateActions({
  candidate,
  commander,
  token,
  spaceId,
  onChanged,
  onError,
}: {
  candidate: VoiceCandidate;
  commander: boolean;
  token: string;
  spaceId: string;
  onChanged: () => Promise<void>;
  onError: (message: string) => void;
}) {
  const [reason, setReason] = useState("");
  const [target, setTarget] = useState("");
  const [draft, setDraft] = useState<ClueDraftCandidate | null>(
    candidate.editable_candidate,
  );
  const [working, setWorking] = useState(false);
  useEffect(() => {
    setDraft(candidate.editable_candidate);
  }, [candidate.editable_candidate]);
  const run = (operation: () => Promise<void>) => {
    setWorking(true);
    void operation()
      .then(onChanged)
      .catch((cause) =>
        onError(cause instanceof Error ? cause.message : "语音候选操作失败"),
      )
      .finally(() => setWorking(false));
  };
  return (
    <div className="mt-2 space-y-2">
      {candidate.returned_for_revision && draft && (
        <div className="space-y-1 border-l-2 border-amber-500 pl-2">
          <label className="block">
            线索摘要{" "}
            <textarea
              className="mt-1 w-full border p-1"
              maxLength={4000}
              value={draft.content_summary ?? ""}
              onChange={(event) =>
                setDraft({ ...draft, content_summary: event.target.value })
              }
            />
          </label>
          <label className="block">
            时间{" "}
            <input
              className="ml-1 w-full border p-1"
              maxLength={100}
              value={draft.occurred_at ?? ""}
              onChange={(event) =>
                setDraft({ ...draft, occurred_at: event.target.value || null })
              }
            />
          </label>
          <label className="block">
            地点{" "}
            <input
              className="ml-1 w-full border p-1"
              maxLength={500}
              value={draft.location_text ?? ""}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  location_text: event.target.value || null,
                })
              }
            />
          </label>
          <label className="block">
            来源说明{" "}
            <input
              className="ml-1 w-full border p-1"
              maxLength={500}
              value={draft.source_text ?? ""}
              onChange={(event) =>
                setDraft({ ...draft, source_text: event.target.value || null })
              }
            />
          </label>
          <button
            className="underline disabled:opacity-50"
            disabled={working}
            type="button"
            onClick={() =>
              run(() =>
                resubmitVoiceCandidate(token, spaceId, candidate.id, draft),
              )
            }
          >
            提交修订
          </button>
        </div>
      )}
      {commander && !candidate.returned_for_revision && (
        <div className="flex flex-wrap items-center gap-2">
          <input
            aria-label="退回或合并原因"
            className="min-w-52 border p-1"
            maxLength={1000}
            placeholder="退回或合并原因"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
          />
          <button
            className="underline disabled:opacity-50"
            disabled={working || !reason.trim()}
            type="button"
            onClick={() =>
              run(() =>
                returnVoiceCandidate(token, spaceId, candidate.id, reason),
              )
            }
          >
            退回提交人
          </button>
          <input
            aria-label="合并目标线索 ID"
            className="min-w-44 border p-1"
            placeholder="目标线索 ID"
            value={target}
            onChange={(event) => setTarget(event.target.value)}
          />
          <button
            className="underline disabled:opacity-50"
            disabled={
              working ||
              !reason.trim() ||
              !target.trim() ||
              !candidate.discoverer_user_id
            }
            type="button"
            onClick={() =>
              run(() =>
                mergeVoiceCandidate(
                  token,
                  spaceId,
                  candidate.id,
                  target.trim(),
                  reason,
                ),
              )
            }
          >
            合并到线索
          </button>
          <span>修改、确认和驳回请在本案件的线索草稿审核区完成。</span>
        </div>
      )}
    </div>
  );
}

function applyEvents(
  events: SpaceEvent[],
  setMessages: Dispatch<SetStateAction<SpaceMessage[]>>,
  acknowledge: (messageId: string) => void,
) {
  for (const event of events)
    if (event.event_type === "message.sent") {
      const payload = event.payload;
      const {
        message_id: id,
        sender_id,
        content,
        sent_at,
        sender_display_name,
      } = payload;
      if (
        typeof id === "string" &&
        typeof sender_id === "string" &&
        typeof content === "string" &&
        typeof sent_at === "string"
      ) {
        acknowledge(id);
        setMessages((current) =>
          current.some((message) => message.id === id)
            ? current
            : [
                {
                  id,
                  sender_id,
                  sender_display_name:
                    typeof sender_display_name === "string"
                      ? sender_display_name
                      : "未知用户",
                  message_type:
                    payload.message_type === "broadcast" ? "broadcast" : "text",
                  content,
                  sent_at,
                  recalled_at: null,
                },
                ...current,
              ],
        );
      }
    }
}
