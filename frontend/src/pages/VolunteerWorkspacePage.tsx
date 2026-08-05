import { Button, Chip, Input, TextArea } from "@heroui/react";
import {
  AlertTriangle,
  Compass,
  LocateFixed,
  MapPin,
  RefreshCw,
  Search,
  Send,
  ShieldCheck,
  Users,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  applyForTask,
  createClue,
  getCase,
  getCaseSummary,
  getTaskNavigation,
  getTaskSafetyBriefing,
  listCasePois,
  listCases,
  listCaseTasks,
  listMyTasks,
  listTaskCollaborationLocations,
  submitTaskFeedback,
  submitTaskLocationReport,
  updateTaskStatus,
} from "../api/cases";
import type {
  CaseDetail,
  CasePois,
  CaseSummary,
  CaseTask,
  TaskCollaborationLocation,
  TaskNavigation,
  TaskSafetyBriefing,
  TaskStatus,
} from "../api/cases";
import { useAuth } from "../auth/useAuth";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../components/ContentState";
import { LocationConfirmationPicker } from "../components/LocationConfirmationPicker";
import { ApiClientError } from "../api/client";

const statusLabels: Record<TaskStatus, string> = {
  pending_claim: "待申请",
  assigned: "待领取",
  accepted: "已接受",
  active: "进行中",
  blocked: "受阻",
  completed: "已完成",
  cancelled: "已取消",
};

const caseStatusLabels: Record<string, string> = {
  active: "进行中",
  resolved: "已找到",
  closed: "已关闭",
};
type Failure = { message: string; retry: (() => void) | null };
type WorkspaceCase = {
  detail: CaseDetail;
  summary: CaseSummary;
  tasks: CaseTask[];
};
type ClueDraft = {
  content: string;
  location: string;
  precision: "exact" | "approximate" | null;
};

function messageFrom(cause: unknown) {
  return cause instanceof Error ? cause.message : "操作未能完成，请稍后重试。";
}

function poiErrorMessage(cause: unknown) {
  if (cause instanceof ApiClientError && cause.status === 409)
    return "当前案件没有可用的任务或已确认公开地点坐标，无法检索周边资源。请使用任务区域文字并联系指挥确认。";
  return messageFrom(cause);
}
function localNow() {
  return new Date().toISOString();
}

function poiSourceLabel(source: string) {
  const labels: Record<string, string> = {
    amap: "高德地图",
    fixed_demo_fallback: "固定演示数据",
  };
  return labels[source] ?? "服务端资源数据";
}

export function VolunteerWorkspacePage() {
  const { token } = useAuth();
  const [cases, setCases] = useState<WorkspaceCase[]>([]);
  const [myTasks, setMyTasks] = useState<CaseTask[]>([]);
  const [navigation, setNavigation] = useState<Record<string, TaskNavigation>>(
    {},
  );
  const [safety, setSafety] = useState<Record<string, TaskSafetyBriefing>>({});
  const [locations, setLocations] = useState<
    Record<string, TaskCollaborationLocation[]>
  >({});
  const [pois, setPois] = useState<Record<string, CasePois>>({});
  const [poiCategory, setPoiCategory] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<Record<string, string>>({});
  const [clueDrafts, setClueDrafts] = useState<Record<string, ClueDraft>>({});
  const [location, setLocation] = useState<
    Record<string, { latitude: string; longitude: string; accuracy: string }>
  >({});
  const [pendingTaskIds, setPendingTaskIds] = useState<Set<string>>(new Set());
  const pendingTaskIdsRef = useRef(new Set<string>());
  const [loading, setLoading] = useState(true);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [notice, setNotice] = useState("");

  const load = useCallback(async () => {
    if (!token) return;
    setLoading(true);
    setFailure(null);
    try {
      const [memberships, assigned] = await Promise.all([
        listCases(token),
        listMyTasks(token),
      ]);
      const volunteerCases = memberships.filter(
        (item) => item.access_role === "volunteer",
      );
      const workspaces = await Promise.all(
        volunteerCases.map(async (item) => {
          const [detail, summary, taskPage] = await Promise.all([
            getCase(token, item.id),
            getCaseSummary(token, item.id),
            listCaseTasks(token, item.id),
          ]);
          return { detail, summary, tasks: taskPage.items };
        }),
      );
      setCases(workspaces);
      setMyTasks(assigned);
    } catch (cause) {
      setFailure({ message: messageFrom(cause), retry: () => void load() });
    } finally {
      setLoading(false);
    }
  }, [token]);
  useEffect(() => {
    void load();
  }, [load]);

  async function run(
    taskId: string,
    action: () => Promise<void>,
    success: string,
  ) {
    if (pendingTaskIdsRef.current.has(taskId)) return;
    pendingTaskIdsRef.current.add(taskId);
    setPendingTaskIds(new Set(pendingTaskIdsRef.current));
    setFailure(null);
    setNotice("");
    try {
      await action();
      setNotice(success);
    } catch (cause) {
      setFailure({
        message: messageFrom(cause),
        retry: () => void run(taskId, action, success),
      });
    } finally {
      pendingTaskIdsRef.current.delete(taskId);
      setPendingTaskIds(new Set(pendingTaskIdsRef.current));
    }
  }
  function statusActions(status: TaskStatus): TaskStatus[] {
    if (status === "assigned") return ["accepted"];
    if (status === "accepted") return ["active"];
    if (status === "active") return ["blocked", "completed"];
    if (status === "blocked") return ["active"];
    return [];
  }
  function refreshTask(updated: CaseTask) {
    setMyTasks((current) =>
      current.map((item) => (item.id === updated.id ? updated : item)),
    );
    setCases((current) =>
      current.map((workspace) => ({
        ...workspace,
        tasks: workspace.tasks.map((item) =>
          item.id === updated.id ? updated : item,
        ),
      })),
    );
  }
  function renderTask(task: CaseTask, assigned: boolean) {
    const busy = pendingTaskIds.has(task.id);
    const taskLocation = location[task.id] ?? {
      latitude: "",
      longitude: "",
      accuracy: "50",
    };
    return (
      <article
        key={task.id}
        className="rounded-md border border-slate-200 bg-white p-4 shadow-sm"
      >
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h3 className="m-0 text-base font-bold text-slate-950">
              {task.title}
            </h3>
            <p className="mb-0 mt-1 text-sm text-slate-700">{task.objective}</p>
          </div>
          <Chip size="sm" variant="soft">
            <Chip.Label>{statusLabels[task.status]}</Chip.Label>
          </Chip>
        </div>
        <dl className="mt-3 grid gap-2 text-sm text-slate-600 sm:grid-cols-2">
          <div>
            <dt className="font-medium text-slate-800">任务区域</dt>
            <dd className="m-0">{task.area_text}</dd>
          </div>
          <div>
            <dt className="font-medium text-slate-800">截止时间</dt>
            <dd className="m-0">{task.due_at}</dd>
          </div>
        </dl>
        {!assigned && task.status === "pending_claim" && (
          <div className="mt-3">
            <Button
              size="sm"
              variant="secondary"
              isDisabled={busy}
              onPress={() => {
                if (token)
                  void run(
                    task.id,
                    async () => {
                      await applyForTask(token, task.id);
                    },
                    "任务申请已提交，等待指挥人员审核。",
                  );
              }}
            >
              <Users size={16} />
              申请协作
            </Button>
          </div>
        )}
        {!assigned && task.status !== "pending_claim" && (
          <p className="mb-0 mt-3 text-xs text-slate-500">
            此任务已有协作人员；通过审核后才能进行任务操作。
          </p>
        )}
        {assigned && (
          <>
            <div className="mt-4 flex flex-wrap gap-2">
              {statusActions(task.status).map((status) => (
                <Button
                  key={status}
                  size="sm"
                  variant={status === "completed" ? "primary" : "secondary"}
                  isDisabled={busy}
                  onPress={() => {
                    if (token)
                      void run(
                        task.id,
                        async () =>
                          refreshTask(
                            await updateTaskStatus(token, task.id, status),
                          ),
                        `任务状态已更新为“${statusLabels[status]}”。`,
                      );
                  }}
                >
                  {statusLabels[status]}
                </Button>
              ))}
              <Button
                size="sm"
                variant="ghost"
                isDisabled={busy}
                onPress={() => {
                  if (token)
                    void run(
                      task.id,
                      async () => {
                        const result = await getTaskNavigation(token, task.id);
                        setNavigation((value) => ({
                          ...value,
                          [task.id]: result,
                        }));
                      },
                      "导航指引已加载。",
                    );
                }}
              >
                <Compass size={16} />
                导航指引
              </Button>
              <Button
                size="sm"
                variant="ghost"
                isDisabled={busy}
                onPress={() => {
                  if (token)
                    void run(
                      task.id,
                      async () => {
                        const result = await getTaskSafetyBriefing(
                          token,
                          task.id,
                        );
                        setSafety((value) => ({ ...value, [task.id]: result }));
                      },
                      "安全提示已加载。",
                    );
                }}
              >
                <ShieldCheck size={16} />
                安全提示
              </Button>
              <Button
                size="sm"
                variant="ghost"
                isDisabled={busy}
                onPress={() => {
                  if (token)
                    void run(
                      task.id,
                      async () => {
                        const result = await listTaskCollaborationLocations(
                          token,
                          task.id,
                        );
                        setLocations((value) => ({
                          ...value,
                          [task.id]: result,
                        }));
                      },
                      "当前协作位置已加载。",
                    );
                }}
              >
                <MapPin size={16} />
                协作位置
              </Button>
            </div>
            {navigation[task.id] && (
              <p className="mt-3 rounded-md border border-blue-200 bg-blue-50 p-3 text-sm text-slate-700">
                {navigation[task.id].route_summary}
              </p>
            )}
            {safety[task.id] && (
              <div className="mt-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-slate-700">
                <div className="font-semibold text-amber-900">
                  <AlertTriangle className="mr-1 inline" size={16} />
                  安全提示
                </div>
                <ul className="mb-0 mt-2 list-disc space-y-1 pl-5">
                  {safety[task.id].notices.map((item, index) => (
                    <li key={`${task.id}-${index}`}>{item}</li>
                  ))}
                </ul>
              </div>
            )}
            {locations[task.id] && (
              <ul className="mt-3 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-900">
                {locations[task.id].length === 0 ? (
                  <li>暂无协作人员的位置上报。</li>
                ) : (
                  locations[task.id].map((item) => (
                    <li key={item.volunteer_user_id}>
                      {item.volunteer_user_id}: {item.latitude.toFixed(5)},{" "}
                      {item.longitude.toFixed(5)}（精度约 {item.accuracy_meters}{" "}
                      米）
                    </li>
                  ))
                )}
              </ul>
            )}
            {task.status === "active" && (
              <div className="mt-4 grid gap-4 border-t border-slate-200 pt-4 lg:grid-cols-2">
                <form
                  className="grid gap-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    if (!token) return;
                    if (
                      !taskLocation.latitude.trim() ||
                      !taskLocation.longitude.trim() ||
                      !taskLocation.accuracy.trim()
                    ) {
                      setFailure({
                        message: "请填写纬度、经度和定位精度。",
                        retry: null,
                      });
                      return;
                    }
                    const latitude = Number(taskLocation.latitude);
                    const longitude = Number(taskLocation.longitude);
                    const accuracy = Number(taskLocation.accuracy);
                    if (
                      !Number.isFinite(latitude) ||
                      latitude < -90 ||
                      latitude > 90 ||
                      !Number.isFinite(longitude) ||
                      longitude < -180 ||
                      longitude > 180 ||
                      !Number.isFinite(accuracy) ||
                      accuracy <= 0
                    ) {
                      setFailure({
                        message: "请填写有效的经纬度和大于 0 的定位精度。",
                        retry: null,
                      });
                      return;
                    }
                    const key = crypto.randomUUID();
                    const payload = {
                      source: "simulated" as const,
                      latitude,
                      longitude,
                      accuracy_meters: accuracy,
                      captured_at: localNow(),
                    };
                    void run(
                      task.id,
                      async () => {
                        await submitTaskLocationReport(
                          token,
                          task.id,
                          payload,
                          key,
                        );
                      },
                      "模拟位置上报已提交。",
                    );
                  }}
                >
                  <h4 className="m-0 text-sm font-semibold text-slate-950">
                    <LocateFixed className="mr-1 inline" size={16} />
                    模拟位置上报
                  </h4>
                  <div className="grid grid-cols-3 gap-2">
                    <Input
                      aria-label="纬度"
                      type="number"
                      value={taskLocation.latitude}
                      onChange={(event) =>
                        setLocation((value) => ({
                          ...value,
                          [task.id]: {
                            ...taskLocation,
                            latitude: event.target.value,
                          },
                        }))
                      }
                    />
                    <Input
                      aria-label="经度"
                      type="number"
                      value={taskLocation.longitude}
                      onChange={(event) =>
                        setLocation((value) => ({
                          ...value,
                          [task.id]: {
                            ...taskLocation,
                            longitude: event.target.value,
                          },
                        }))
                      }
                    />
                    <Input
                      aria-label="定位精度（米）"
                      type="number"
                      value={taskLocation.accuracy}
                      onChange={(event) =>
                        setLocation((value) => ({
                          ...value,
                          [task.id]: {
                            ...taskLocation,
                            accuracy: event.target.value,
                          },
                        }))
                      }
                    />
                  </div>
                  <Button
                    type="submit"
                    size="sm"
                    variant="secondary"
                    isDisabled={busy}
                  >
                    提交位置
                  </Button>
                </form>
                <form
                  className="grid gap-2"
                  onSubmit={(event) => {
                    event.preventDefault();
                    if (!token || !feedback[task.id]?.trim()) return;
                    const key = crypto.randomUUID();
                    const payload = {
                      content: feedback[task.id].trim(),
                      occurred_at: localNow(),
                      location_text: task.area_text,
                      location_precision: "approximate" as const,
                    };
                    void run(
                      task.id,
                      async () => {
                        await submitTaskFeedback(token, task.id, payload, key);
                        setFeedback((value) => ({ ...value, [task.id]: "" }));
                      },
                      "执行反馈已提交审核。",
                    );
                  }}
                >
                  <h4 className="m-0 text-sm font-semibold text-slate-950">
                    执行反馈
                  </h4>
                  <TextArea
                    aria-label="执行反馈"
                    value={feedback[task.id] ?? ""}
                    rows={3}
                    maxLength={4000}
                    onChange={(event) =>
                      setFeedback((value) => ({
                        ...value,
                        [task.id]: event.target.value,
                      }))
                    }
                    fullWidth
                  />
                  <Button
                    type="submit"
                    size="sm"
                    variant="secondary"
                    isDisabled={busy || !feedback[task.id]?.trim()}
                  >
                    提交反馈
                  </Button>
                </form>
              </div>
            )}
          </>
        )}
      </article>
    );
  }

  const myTaskIds = new Set(myTasks.map((task) => task.id));
  return (
    <main className="mx-auto w-full max-w-6xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 flex flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <span className="mb-1 block text-xs font-semibold text-slate-500">
            志愿协作
          </span>
          <h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">
            我的任务
          </h1>
          <p className="mb-0 mt-1 text-sm text-slate-600">
            案件信息、开放任务、待核查线索与周边资源。
          </p>
        </div>
        <Button
          size="sm"
          variant="ghost"
          isDisabled={loading}
          onPress={() => void load()}
        >
          <RefreshCw size={16} />
          刷新
        </Button>
      </header>
      {notice && (
        <p className="mb-4 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
          {notice}
        </p>
      )}
      {failure && !loading && (
        <div className="mb-4">
          <ErrorState
            message={failure.message}
            onRetry={failure.retry ?? undefined}
          />
        </div>
      )}
      {loading ? (
        <LoadingState label="正在加载协作工作台" />
      ) : cases.length === 0 ? (
        <EmptyState
          icon={Users}
          title="暂无可协作案件"
          description="被添加为志愿者的案件会显示在这里，任务分配前也可查看。"
        />
      ) : (
        <section className="grid gap-6">
          {cases.map((workspace) => {
            const clueDraft = clueDrafts[workspace.detail.id] ?? {
              content: "",
              location: "",
              precision: null,
            };
            const assignedTasks = workspace.tasks.filter((task) =>
              myTaskIds.has(task.id),
            );
            const openTasks = workspace.tasks.filter(
              (task) =>
                task.status === "pending_claim" && !myTaskIds.has(task.id),
            );
            const category = poiCategory[workspace.detail.id] ?? "hospital";
            return (
              <article
                key={workspace.detail.id}
                className="rounded-lg border border-slate-200 bg-white p-5 shadow-sm"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <span className="text-xs font-semibold text-slate-500">
                      {workspace.detail.case_code}
                    </span>
                    <h2 className="m-0 text-xl font-bold text-slate-950">
                      {workspace.detail.elder_profile.display_name}
                    </h2>
                  </div>
                  <Chip size="sm" variant="soft">
                    <Chip.Label>
                      {caseStatusLabels[workspace.detail.status] ?? "状态未知"}
                    </Chip.Label>
                  </Chip>
                </div>
                <div className="mt-4 grid gap-4 text-sm text-slate-700 lg:grid-cols-3">
                  <div>
                    <h3 className="m-0 text-sm font-semibold text-slate-950">
                      家属联系方式
                    </h3>
                    <p className="mb-0 mt-1">
                      {workspace.detail.family_contact_emails?.join(", ") ||
                        "暂未提供家属联系方式。"}
                    </p>
                  </div>
                  <div>
                    <h3 className="m-0 text-sm font-semibold text-slate-950">
                      健康注意事项
                    </h3>
                    <p className="mb-0 mt-1">
                      {workspace.detail.elder_profile.health_notes ||
                        "暂无健康注意事项。"}
                    </p>
                  </div>
                  <div>
                    <h3 className="m-0 text-sm font-semibold text-slate-950">
                      案件摘要
                    </h3>
                    <p className="mb-0 mt-1">
                      {workspace.summary.last_confirmed_information?.content ||
                        "暂无已确认的摘要信息。"}
                    </p>
                  </div>
                </div>
                <div className="mt-5 grid gap-5 lg:grid-cols-2">
                  <section>
                    <h3 className="m-0 text-sm font-semibold text-slate-950">
                      已确认线索与我的上报
                    </h3>
                    <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-slate-700">
                      {workspace.detail.clues.length === 0 ? (
                        <li>暂无可查看的线索。</li>
                      ) : (
                        workspace.detail.clues.slice(0, 8).map((clue) => (
                          <li key={clue.id}>
                            {clue.content}
                            {clue.is_own_submission && (
                              <span className="ml-1 text-xs text-slate-500">
                                （由我上报）
                              </span>
                            )}
                          </li>
                        ))
                      )}
                    </ul>
                    <form
                      className="mt-4 grid gap-2 rounded-md border border-slate-200 bg-slate-50 p-3"
                      onSubmit={(event) => {
                        event.preventDefault();
                        if (!token || !clueDraft.content.trim()) return;
                        void run(
                          `case-${workspace.detail.id}-clue`,
                          async () => {
                            await createClue(token, workspace.detail.id, {
                              source: "volunteer",
                              source_type: "field_report",
                              content: clueDraft.content.trim(),
                              occurred_at: localNow(),
                              location_text: clueDraft.location.trim() || null,
                              location_precision: clueDraft.location.trim()
                                ? (clueDraft.precision ?? "approximate")
                                : null,
                            });
                            setClueDrafts((value) => ({
                              ...value,
                              [workspace.detail.id]: {
                                content: "",
                                location: "",
                                precision: null,
                              },
                            }));
                            await load();
                          },
                          "现场观察已作为待审核线索提交。",
                        );
                      }}
                    >
                      <h4 className="m-0 text-sm font-semibold text-slate-950">
                        <Send className="mr-1 inline" size={16} />
                        提交现场观察以供审核
                      </h4>
                      <TextArea
                        aria-label="观察内容"
                        value={clueDraft.content}
                        maxLength={4000}
                        rows={3}
                        placeholder="请只描述亲眼观察到的内容；提交后将等待人工审核。"
                        onChange={(event) =>
                          setClueDrafts((value) => ({
                            ...value,
                            [workspace.detail.id]: {
                              ...clueDraft,
                              content: event.target.value,
                            },
                          }))
                        }
                        fullWidth
                      />
                      <Input
                        aria-label="观察地点"
                        value={clueDraft.location}
                        maxLength={500}
                        placeholder="可选：地点文字说明"
                        onChange={(event) =>
                          setClueDrafts((value) => ({
                            ...value,
                            [workspace.detail.id]: {
                              ...clueDraft,
                              location: event.target.value,
                              precision: "approximate",
                            },
                          }))
                        }
                        fullWidth
                      />
                      <LocationConfirmationPicker
                        onConfirm={(location) =>
                          setClueDrafts((value) => ({
                            ...value,
                            [workspace.detail.id]: {
                              ...clueDraft,
                              location: location.address,
                              precision: location.precision,
                            },
                          }))
                        }
                        onClear={() =>
                          setClueDrafts((value) => ({
                            ...value,
                            [workspace.detail.id]: {
                              ...clueDraft,
                              location: "",
                              precision: null,
                            },
                          }))
                        }
                      />
                      <Button
                        type="submit"
                        size="sm"
                        variant="secondary"
                        isDisabled={
                          pendingTaskIds.has(
                            `case-${workspace.detail.id}-clue`,
                          ) || !clueDraft.content.trim()
                        }
                      >
                        提交审核
                      </Button>
                    </form>
                  </section>
                  <section>
                    <h3 className="m-0 text-sm font-semibold text-slate-950">
                      附近资源
                    </h3>
                    <p className="mb-0 mt-1 text-xs text-slate-600">
                      服务端会根据任务区域或已确认的公开地点选择授权中心；
                      不会使用浏览器提供的位置作为检索中心。
                    </p>
                    <div className="mt-2 flex flex-wrap gap-2">
                      <select
                        aria-label="附近资源类别"
                        className="min-h-9 rounded-md border border-slate-300 bg-white px-2 text-sm"
                        value={category}
                        onChange={(event) =>
                          setPoiCategory((value) => ({
                            ...value,
                            [workspace.detail.id]: event.target.value,
                          }))
                        }
                      >
                        <option value="hospital">医院</option>
                        <option value="police">公安机关</option>
                        <option value="transit">交通站点</option>
                        <option value="market">市场</option>
                        <option value="community_service">社区服务</option>
                      </select>
                      <Button
                        size="sm"
                        variant="secondary"
                        isDisabled={pendingTaskIds.has(
                          `case-${workspace.detail.id}-pois`,
                        )}
                        onPress={() => {
                          if (token)
                            void run(
                              `case-${workspace.detail.id}-pois`,
                              async () => {
                                const result = await listCasePois(
                                  token,
                                  workspace.detail.id,
                                  category,
                                ).catch((cause) => {
                                  throw new Error(poiErrorMessage(cause));
                                });
                                setPois((value) => ({
                                  ...value,
                                  [workspace.detail.id]: result,
                                }));
                              },
                              "附近资源已加载。",
                            );
                        }}
                      >
                        <Search size={16} />
                        搜索附近资源
                      </Button>
                    </div>
                    {pois[workspace.detail.id] && (
                      <div className="mt-3 rounded-md border border-slate-200 p-3 text-sm text-slate-700">
                        <p className="m-0 text-xs text-slate-500">
                          数据来源：
                          {poiSourceLabel(pois[workspace.detail.id].source)}
                          {pois[workspace.detail.id].fallback_message
                            ? ` · ${pois[workspace.detail.id].fallback_message}`
                            : ""}
                        </p>
                        <ul className="mb-0 mt-2 list-disc space-y-1 pl-5">
                          {pois[workspace.detail.id].items.map((poi) => (
                            <li key={poi.id}>
                              {poi.name}
                              {poi.address ? ` — ${poi.address}` : ""}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </section>
                </div>
                <section className="mt-5 border-t border-slate-200 pt-5">
                  <h3 className="m-0 text-base font-semibold text-slate-950">
                    我的协作任务
                  </h3>
                  <div className="mt-3 grid gap-3 xl:grid-cols-2">
                    {assignedTasks.length === 0 ? (
                      <p className="mb-0 text-sm text-slate-500">
                        你尚未参与任何任务。
                      </p>
                    ) : (
                      assignedTasks.map((task) => renderTask(task, true))
                    )}
                  </div>
                </section>
                <section className="mt-5 border-t border-slate-200 pt-5">
                  <h3 className="m-0 text-base font-semibold text-slate-950">
                    开放任务
                  </h3>
                  <p className="mb-0 mt-1 text-xs text-slate-600">
                    这些常设或现场任务正在等待志愿者申请和指挥人员审核。
                  </p>
                  <div className="mt-3 grid gap-3 xl:grid-cols-2">
                    {openTasks.length === 0 ? (
                      <p className="mb-0 text-sm text-slate-500">
                        当前暂无开放任务。
                      </p>
                    ) : (
                      openTasks.map((task) => renderTask(task, false))
                    )}
                  </div>
                </section>
              </article>
            );
          })}
        </section>
      )}
    </main>
  );
}
