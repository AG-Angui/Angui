import { AlertDialog, Button, Chip, Input, TextArea } from "@heroui/react";
import {
  CheckCircle2,
  ChevronRight,
  CirclePlus,
  FileSearch,
  MapPin,
  RefreshCw,
  Send,
  UserPlus,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import {
  addCaseMember,
  acceptCommandCase,
  applyForTask,
  createCasePlace,
  createClueDraft,
  createArchiveDraft,
  createSummaryDraft,
  createCaseTask,
  getCaseMapView,
  getCaseSummary,
  getLatestSummaryDraft,
  getCasePublicProgress,
  listCasePois,
  listCaseMembers,
  listCaseTasks,
  createClue,
  getCase,
  getCaseResourceConfiguration,
  listCaseClues,
  listClueDrafts,
  listSummaryDraftVersions,
  diffSummaryDraftVersions,
  listCases,
  listCommandIntake,
  reviewClue,
  reviewClueDraft,
  reviewSummaryDraft,
  updateCaseStatus,
  updateElderProfile,
  updateTaskStatus,
  submitTaskFeedback,
  uploadCaseAttachment,
} from "../api/cases";
import type {
  CaseDetail,
  CaseListItem,
  CaseRole,
  CaseResourceConfiguration,
  CaseStatus,
  CasePois,
  CaseMapItem,
  CaseMapView,
  CaseSummary,
  CaseMember,
  CaseTask,
  CreateTaskPayload,
  CasePublicProgress,
  Clue,
  ClueDraft,
  ClueReviewStatus,
  ClueSourceType,
  ClueStatus,
  CreateCasePlacePayload,
  LocationPrecision,
  PublicClueSourceType,
  SummaryDraft,
  SummaryDraftDiff,
  PlaceType,
  PlaceVisibility,
} from "../api/cases";
import { ApiClientError } from "../api/client";
import { useAuth } from "../auth/useAuth";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../components/ContentState";
import { FamilyIntakeForm } from "./FamilyIntakeForm";

type WorkspaceMode = "family" | "commander" | "volunteer";
type ReviewDraft = {
  reason: string;
  relatedClueId: string;
  relatedClueQuery: string;
};
type ClueQueueFilters = {
  status: ClueStatus | "";
  sourceType: ClueSourceType | "";
  query: string;
  sort: "created_at" | "occurred_at";
  order: "asc" | "desc";
  page: number;
};
const defaultClueQueueFilters: ClueQueueFilters = {
  status: "pending_review",
  sourceType: "",
  query: "",
  sort: "created_at",
  order: "desc",
  page: 1,
};

const workspaceCopy: Record<WorkspaceMode, { context: string; title: string }> =
  {
    family: { context: "家属端", title: "走失求助" },
    commander: { context: "指挥端", title: "案件指挥" },
    volunteer: { context: "志愿者端", title: "协作案件" },
  };

const statusLabels: Record<string, string> = {
  active: "进行中",
  resolved: "已找到",
  closed: "已关闭",
  pending_review: "待审核",
  needs_verification: "待核实",
  confirmed: "已确认",
  rejected: "已排除",
  expired: "已失效",
  duplicate: "重复",
  conflicting: "冲突",
  insufficient_information: "信息不足",
};

const caseRoleLabels: Record<CaseRole, string> = {
  family: "家属",
  commander: "指挥人员",
  volunteer: "志愿者",
};

const placeTypeLabels: Record<string, string> = {
  frequent: "常去地点",
  key_location: "关键地点",
  last_seen_context: "最后出现相关",
  medical: "医疗",
  shelter: "临时安置",
  other: "其他",
};

export function CaseWorkspacePage({ mode }: { mode: WorkspaceMode }) {
  const { token, user } = useAuth();
  const navigate = useNavigate();
  const { caseId: routeCaseId } = useParams();
  const [cases, setCases] = useState<CaseListItem[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<CaseDetail | null>(null);
  const [resourceConfiguration, setResourceConfiguration] =
    useState<CaseResourceConfiguration | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isDetailLoading, setIsDetailLoading] = useState(false);
  const [listError, setListError] = useState("");
  const [detailError, setDetailError] = useState("");
  const [notice, setNotice] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const detailRequestVersion = useRef(0);

  const loadCases = useCallback(
    async (preferredId?: string) => {
      if (!token) return;
      setIsLoading(true);
      setListError("");
      try {
        const items = await listCases(token);
        setCases(items);
        setSelectedId((currentId) => {
          const nextId =
            preferredId ??
            (mode === "commander" ? routeCaseId ?? null : currentId ?? items[0]?.id ?? null);
          if (!nextId) setDetail(null);
          return nextId;
        });
      } catch (cause) {
        setListError(messageFrom(cause));
      } finally {
        setIsLoading(false);
      }
    },
    [mode, routeCaseId, token],
  );

  const loadDetail = useCallback(
    async (caseId: string) => {
      const requestVersion = detailRequestVersion.current + 1;
      detailRequestVersion.current = requestVersion;
      if (!token) return;
      setIsDetailLoading(true);
      setDetailError("");
      try {
        const [nextDetail, nextResourceConfiguration] = await Promise.all([
          getCase(token, caseId),
          getCaseResourceConfiguration(token, caseId),
        ]);
        if (requestVersion !== detailRequestVersion.current) return;
        setDetail(nextDetail);
        setResourceConfiguration(nextResourceConfiguration);
      } catch (cause) {
        if (requestVersion !== detailRequestVersion.current) return;
        setDetailError(messageFrom(cause));
        setDetail(null);
        setResourceConfiguration(null);
      } finally {
        if (requestVersion === detailRequestVersion.current)
          setIsDetailLoading(false);
      }
    },
    [token],
  );

  useEffect(() => {
    void loadCases();
  }, [loadCases]);

  useEffect(() => {
    if (mode === "commander") setSelectedId(routeCaseId ?? null);
  }, [mode, routeCaseId]);

  useEffect(() => {
    if (selectedId) {
      void loadDetail(selectedId);
      return;
    }
    detailRequestVersion.current += 1;
    setDetail(null);
    setResourceConfiguration(null);
    setDetailError("");
    setIsDetailLoading(false);
  }, [loadDetail, selectedId]);

  const pendingCount = useMemo(
    () =>
      detail?.clues.filter((clue) => clue.status === "pending_review").length ??
      0,
    [detail],
  );
  const copy = workspaceCopy[mode];
  const canCreateCase = user?.account_type === "member";
  const isCommandCaseDetail = mode === "commander" && Boolean(routeCaseId);

  if (isCommandCaseDetail) {
    return (
      <div className="mx-auto w-full max-w-[1440px] px-4 py-6 sm:px-6 lg:px-8">
        <div className="grid gap-6 lg:grid-cols-[248px_minmax(0,1fr)]">
          <aside className="lg:sticky lg:top-6 lg:self-start">
            <nav className="border border-slate-200 bg-white p-4" aria-label="案件详情导航">
              <Link className="text-sm font-medium text-brand-700 hover:underline" to="/command">
                返回案件列表
              </Link>
              <div className="mt-5 border-y border-slate-200 py-4">
                <span className="block text-xs text-slate-500">当前位置</span>
                <strong className="mt-1 block text-sm text-slate-950">
                  {detail?.elder_profile.display_name ?? "正在加载案件"}
                </strong>
                <span className="mt-1 block text-xs text-slate-500">
                  {detail?.case_code ?? routeCaseId}
                </span>
                {detail && (
                  <span className="mt-2 inline-block text-xs font-medium text-slate-700">
                    {statusLabels[detail.status]}
                  </span>
                )}
              </div>
              <div className="mt-4 grid gap-1 text-sm">
                <a className="rounded-md px-3 py-2 text-slate-700 hover:bg-brand-50 hover:text-brand-700" href="#case-tasks">任务与审核</a>
                <a className="rounded-md px-3 py-2 text-slate-700 hover:bg-brand-50 hover:text-brand-700" href="#case-clues">态势与线索</a>
                <a className="rounded-md px-3 py-2 text-slate-700 hover:bg-brand-50 hover:text-brand-700" href="#case-profile">案件资料</a>
                <a className="rounded-md px-3 py-2 text-slate-700 hover:bg-brand-50 hover:text-brand-700" href="#case-members">协作与成员</a>
              </div>
            </nav>
          </aside>
          <main id="case-detail-content" className="min-w-0 border border-slate-200 bg-white">
            {isDetailLoading ? (
              <LoadingState label="正在加载案件详情" />
            ) : detailError ? (
              <ErrorState message={detailError} onRetry={() => selectedId && void loadDetail(selectedId)} />
            ) : detail && resourceConfiguration ? (
              <CaseDetailView
                detail={detail}
                resourceConfiguration={resourceConfiguration}
                pendingCount={pendingCount}
                onChanged={async (message) => {
                  setNotice(message);
                  await loadDetail(detail.id);
                  await loadCases(detail.id);
                }}
              />
            ) : null}
          </main>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-6 flex min-h-14 flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <span className="mb-1 block text-xs font-semibold text-slate-500">
            {copy.context}
          </span>
          <h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">
            {copy.title}
          </h1>
        </div>
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onPress={() => void loadCases()}>
            <RefreshCw size={16} />
            刷新
          </Button>
          {canCreateCase && mode !== "volunteer" && (
            <Button
              size="sm"
              variant="primary"
              onPress={() => setShowCreate((value) => !value)}
            >
              <CirclePlus size={16} />
              新建案件
            </Button>
          )}
        </div>
      </header>

      {listError && cases.length > 0 && (
        <Message tone="error">{listError}</Message>
      )}
      {notice && <Message tone="success">{notice}</Message>}

      {mode === "commander" && (
        <CommandIntakePanel
          token={token}
          onAccepted={async () => {
            await loadCases();
          }}
        />
      )}

      {showCreate && canCreateCase && mode !== "volunteer" && (
        <FamilyIntakeForm
          onCancel={() => setShowCreate(false)}
          onConfirmed={async (caseId, caseCode) => {
            setShowCreate(false);
            setNotice(`案件 ${caseCode} 已由家属人工确认创建`);
            await loadCases(caseId);
          }}
        />
      )}

      <div className={`mt-5 min-h-[560px] overflow-hidden border-y border-slate-200 bg-white ${mode === "commander" ? "" : "grid lg:grid-cols-[310px_minmax(0,1fr)]"}`}>
        <section
          className="border-b border-slate-200 lg:border-r lg:border-b-0"
          aria-label="案件列表"
        >
          <div className="flex min-h-16 items-center justify-between border-b border-slate-200 px-4 py-3">
            <strong className="text-sm text-slate-950">可访问案件</strong>
            <Chip size="sm" variant="soft">
              <Chip.Label>{cases.length}</Chip.Label>
            </Chip>
          </div>
          {isLoading ? (
            <LoadingState label="正在加载可访问案件" />
          ) : listError && cases.length === 0 ? (
            <ErrorState message={listError} onRetry={() => void loadCases()} />
          ) : cases.length === 0 ? (
            <EmptyState
              title="暂无案件"
              description="新建案件后，会显示在这里。"
            />
          ) : (
            <div className="divide-y divide-slate-100">
              {cases.map((item) => (
                <button
                  type="button"
                  key={item.id}
                  onClick={() => {
                    if (mode === "commander") {
                      navigate(`/command/cases/${item.id}`);
                      return;
                    }
                    setSelectedId(item.id);
                  }}
                  aria-pressed={selectedId === item.id}
                  className={`flex min-h-20 w-full items-center gap-3 px-4 py-3 text-left transition-colors ${
                    selectedId === item.id ? "bg-brand-50" : "hover:bg-slate-50"
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <strong className="truncate text-sm text-slate-950">
                        {item.display_name}
                      </strong>
                      <Chip size="sm" variant="soft">
                        <Chip.Label>{statusLabels[item.status]}</Chip.Label>
                      </Chip>
                    </div>
                    <span className="mt-1 block text-xs text-slate-500">
                      权限：{caseRoleLabels[item.access_role]}
                    </span>
                    <span className="mt-1 block truncate text-xs text-slate-500">
                      {item.case_code}
                    </span>
                    <span className="mt-0.5 block truncate text-xs text-slate-500">
                      {item.last_seen_location ?? "地点待补充"}
                    </span>
                  </div>
                  <ChevronRight size={17} className="shrink-0 text-slate-400" />
                </button>
              ))}
            </div>
          )}
        </section>

        {mode !== "commander" && <section className="min-w-0">
          {isDetailLoading ? (
            <LoadingState label="正在加载案件详情" />
          ) : detailError ? (
            <ErrorState
              message={detailError}
              onRetry={() => selectedId && void loadDetail(selectedId)}
            />
          ) : detail && resourceConfiguration ? (
            <CaseDetailView
              detail={detail}
              resourceConfiguration={resourceConfiguration}
              pendingCount={pendingCount}
              onChanged={async (message) => {
                setNotice(message);
                await loadDetail(detail.id);
                await loadCases(detail.id);
              }}
            />
          ) : (
            <div className="flex min-h-96 flex-col items-center justify-center px-6 text-center">
              <EmptyState icon={FileSearch} title="选择一个案件查看详情" />
            </div>
          )}
        </section>}
      </div>
    </div>
  );
}

function CaseDetailView({
  detail,
  resourceConfiguration,
  pendingCount,
  onChanged,
}: {
  detail: CaseDetail;
  resourceConfiguration: CaseResourceConfiguration;
  pendingCount: number;
  onChanged: (message: string) => Promise<void>;
}) {
  const { token } = useAuth();
  const [clueContent, setClueContent] = useState("");
  const [clueLocation, setClueLocation] = useState("");
  const [clueOccurredAt, setClueOccurredAt] = useState("");
  const [clueSourceType, setClueSourceType] =
    useState<PublicClueSourceType>("manual_report");
  const [clueRawReference, setClueRawReference] = useState("");
  const [clueLocationPrecision, setClueLocationPrecision] = useState<
    LocationPrecision | ""
  >("");
  const [clueNextAction, setClueNextAction] = useState("");
  const [linkedAttachmentIds, setLinkedAttachmentIds] = useState<string[]>([]);
  const [reviewDrafts, setReviewDrafts] = useState<Record<string, ReviewDraft>>(
    {},
  );
  const [clueQueue, setClueQueue] = useState<{
    items: Clue[];
    total: number;
    page: number;
    pageSize: number;
  }>({
    items: [],
    total: 0,
    page: 1,
    pageSize: 25,
  });
  const [queueFilters, setQueueFilters] = useState<ClueQueueFilters>(
    defaultClueQueueFilters,
  );
  const [isQueueLoading, setIsQueueLoading] = useState(false);
  const [queueError, setQueueError] = useState("");
  const queueRequestVersion = useRef(0);
  const [place, setPlace] = useState<CreateCasePlacePayload>({
    name: "",
    place_type: "",
    address: "",
    longitude: null,
    latitude: null,
    visibility: "confirmed",
  });
  const [attachment, setAttachment] = useState<File | null>(null);
  const [nextStatus, setNextStatus] = useState<CaseStatus>(detail.status);
  const [memberEmail, setMemberEmail] = useState("");
  const [memberRole, setMemberRole] = useState<CaseRole>("volunteer");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const isCommander = detail.access_role === "commander";
  const allowedMemberRoles: CaseRole[] = isCommander
    ? ["family", "commander", "volunteer"]
    : ["family", "commander"];
  const selectedMemberRole = allowedMemberRoles.includes(memberRole)
    ? memberRole
    : allowedMemberRoles[0];
  const canEditElderProfile = detail.access_role === "family" || isCommander;
  const canSubmitPlace = detail.access_role === "family" || isCommander;
  const placeTypes = resourceConfiguration.case_place_types;
  const statusOptions: CaseStatus[] =
    detail.status === "active"
      ? ["active", "resolved", "closed"]
      : detail.status === "resolved"
        ? ["resolved", "active", "closed"]
        : ["closed"];

  const loadClueQueue = useCallback(async () => {
    const requestVersion = queueRequestVersion.current + 1;
    queueRequestVersion.current = requestVersion;
    if (!token) {
      setClueQueue({ items: [], total: 0, page: 1, pageSize: 25 });
      setQueueError("");
      setIsQueueLoading(false);
      return;
    }
    if (!isCommander) return;
    setClueQueue({
      items: [],
      total: 0,
      page: queueFilters.page,
      pageSize: 25,
    });
    setIsQueueLoading(true);
    setQueueError("");
    try {
      const page = await listCaseClues(token, detail.id, {
        page: queueFilters.page,
        page_size: 25,
        status: queueFilters.status || undefined,
        source_type: queueFilters.sourceType || undefined,
        q: queueFilters.query.trim() || undefined,
        sort: queueFilters.sort,
        order: queueFilters.order,
      });
      if (requestVersion !== queueRequestVersion.current) return;
      setClueQueue({
        items: page.items,
        total: page.total,
        page: page.page,
        pageSize: page.page_size,
      });
    } catch (cause) {
      if (requestVersion !== queueRequestVersion.current) return;
      setQueueError(messageFrom(cause));
    } finally {
      if (requestVersion === queueRequestVersion.current)
        setIsQueueLoading(false);
    }
  }, [detail.id, isCommander, queueFilters, token]);

  const visibleClues = isCommander ? clueQueue.items : detail.clues;

  useEffect(() => setNextStatus(detail.status), [detail.status]);
  useEffect(() => {
    setPlace((current) =>
      placeTypes.includes(current.place_type)
        ? current
        : { ...current, place_type: placeTypes[0] ?? "" },
    );
  }, [placeTypes]);
  useEffect(() => {
    if (isCommander) {
      void loadClueQueue();
      return;
    }
    queueRequestVersion.current += 1;
    setClueQueue({ items: [], total: 0, page: 1, pageSize: 25 });
    setQueueError("");
    setIsQueueLoading(false);
  }, [isCommander, loadClueQueue]);

  async function run(
    key: string,
    action: () => Promise<unknown>,
    message: string,
  ) {
    setBusy(key);
    setError("");
    try {
      await action();
      await onChanged(message);
      await loadClueQueue();
      return true;
    } catch (cause) {
      setError(messageFrom(cause));
      return false;
    } finally {
      setBusy("");
    }
  }

  return (
    <div>
      <header className="border-b border-slate-200 px-5 py-4 sm:px-6">
        <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="m-0 text-xl font-bold text-slate-950">
                {detail.elder_profile.display_name}
              </h2>
              <Chip size="sm" variant="soft">
                <Chip.Label>{statusLabels[detail.status]}</Chip.Label>
              </Chip>
              {pendingCount > 0 && (
                <Chip size="sm" variant="soft">
                  <Chip.Label>{pendingCount} 条待审核</Chip.Label>
                </Chip>
              )}
            </div>
            <span className="mt-1 block text-xs text-slate-500">
              {detail.case_code}
            </span>
          </div>
          <span className="text-xs text-slate-500">
            权限：{caseRoleLabels[detail.access_role]}
          </span>
        </div>
      </header>

      <RoleActionPanel
        accessRole={detail.access_role}
        caseStatus={detail.status}
        pendingCount={pendingCount}
        hasProfileGaps={
          !detail.elder_profile.last_seen_location ||
          !detail.elder_profile.last_seen_at ||
          !detail.elder_profile.physical_description
        }
      />
      <nav
        className="flex flex-wrap gap-x-4 gap-y-2 border-b border-slate-200 bg-white px-5 py-3 text-sm font-medium sm:px-6"
        aria-label="案件工作区导航"
      >
        <a
          className="text-brand-700 underline-offset-4 hover:underline"
          href={detail.access_role === "family" ? "#case-actions" : "#task-board"}
        >
          当前行动
        </a>
        <a className="text-brand-700 underline-offset-4 hover:underline" href="#case-clues">
          线索与地点
        </a>
        <a className="text-brand-700 underline-offset-4 hover:underline" href="#case-profile">
          案件资料
        </a>
      </nav>

      {detail.access_role !== "family" && (
        <TaskBoard detail={detail} token={token} />
      )}
      <CaseCollaborationPanel detail={detail} token={token} />
      <CaseSituationPanel detail={detail} token={token} />

      <section
        id="case-profile"
        className="grid gap-x-8 gap-y-4 border-b border-slate-200 px-5 py-5 sm:grid-cols-2 sm:px-6 lg:grid-cols-3"
        aria-label="老人资料"
      >
        <Info
          label="最后出现地点"
          value={detail.elder_profile.last_seen_location}
          icon={<MapPin size={16} />}
        />
        <Info
          label="最后出现时间"
          value={formatDate(detail.elder_profile.last_seen_at)}
        />
        <Info
          label="年龄"
          value={
            detail.elder_profile.age == null
              ? null
              : `${detail.elder_profile.age} 岁`
          }
        />
        <Info label="体貌" value={detail.elder_profile.physical_description} />
        <Info label="衣着" value={detail.elder_profile.clothing_description} />
        {detail.elder_profile.health_notes && (
          <Info label="健康注意" value={detail.elder_profile.health_notes} />
        )}
      </section>

      {canEditElderProfile && (
        <details id="case-profile-editor" className="border-b border-slate-200 bg-brand-50/30">
          <summary className="cursor-pointer px-5 py-4 text-sm font-semibold text-slate-900 sm:px-6">
            补充或更正人物资料
          </summary>
          <ElderProfileEditor
            detail={detail}
            token={token}
            busy={busy}
            onSave={(payload) =>
              run(
                "elder-profile",
                () => updateElderProfile(token!, detail.id, payload),
                "人物摘要已更新",
              )
            }
          />
        </details>
      )}

      {(isCommander || detail.access_role === "family") && (
        <details id="case-members" className="border-b border-slate-200 bg-slate-50">
          <summary className="cursor-pointer px-5 py-4 text-sm font-semibold text-slate-900 sm:px-6">
            案件状态与成员管理
          </summary>
          <section className="grid gap-5 px-5 pb-5 sm:px-6 lg:grid-cols-2">
            {isCommander && (
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  if (!token) return;
                  void run(
                    "status",
                    () => updateCaseStatus(token, detail.id, nextStatus),
                    "案件状态已更新",
                  );
                }}
              >
                <h3
                  id="case-status-title"
                  className="m-0 text-sm font-bold text-slate-950"
                >
                  案件状态
                </h3>
                <div className="mt-3 flex gap-2">
                  <select
                    aria-labelledby="case-status-title"
                    className="min-h-10 flex-1 rounded-md border border-slate-300 bg-white px-3 text-sm"
                    value={nextStatus}
                    onChange={(event) =>
                      setNextStatus(event.target.value as CaseStatus)
                    }
                    disabled={detail.status === "closed"}
                  >
                    {statusOptions.map((status) => (
                      <option key={status} value={status}>
                        {statusLabels[status]}
                      </option>
                    ))}
                  </select>
                  <Button
                    type="submit"
                    size="sm"
                    variant="secondary"
                    isDisabled={busy === "status" || nextStatus === detail.status}
                  >
                    保存
                  </Button>
                </div>
              </form>
            )}

            <form
              onSubmit={(event) => {
                event.preventDefault();
                if (!token) return;
                void run(
                  "member",
                  () =>
                    addCaseMember(
                      token,
                      detail.id,
                      memberEmail.trim(),
                      selectedMemberRole,
                    ),
                  "案件成员已添加",
                ).then((succeeded) => {
                  if (succeeded) setMemberEmail("");
                });
              }}
            >
            <h3 className="m-0 text-sm font-bold text-slate-950">添加成员</h3>
            <div className="mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_120px_auto]">
              <Input
                type="email"
                value={memberEmail}
                maxLength={320}
                onChange={(event) => setMemberEmail(event.target.value)}
                placeholder="成员邮箱"
                fullWidth
                required
              />
              <select
                aria-label="成员角色"
                className="min-h-10 rounded-md border border-slate-300 bg-white px-3 text-sm"
                value={selectedMemberRole}
                onChange={(event) =>
                  setMemberRole(event.target.value as CaseRole)
                }
              >
                <option value="family">家属</option>
                <option value="commander">指挥</option>
                {isCommander && <option value="volunteer">志愿者</option>}
              </select>
              <Button
                type="submit"
                size="sm"
                variant="secondary"
                isDisabled={busy === "member"}
                isIconOnly
                aria-label="添加案件成员"
              >
                <UserPlus size={17} />
              </Button>
            </div>
            </form>
          </section>
        </details>
      )}

      {error && (
        <div className="px-5 pt-4 sm:px-6">
          <Message tone="error">{error}</Message>
        </div>
      )}

      <section id="case-clues" className="px-5 py-5 sm:px-6" aria-labelledby="clues-title">
        <div className="flex items-center justify-between gap-3">
          <h3
            id="clues-title"
            className="m-0 text-base font-bold text-slate-950"
          >
            线索
          </h3>
          <Chip size="sm" variant="soft">
            <Chip.Label>{detail.clues.length} 条可见</Chip.Label>
          </Chip>
        </div>

        {isCommander && (
          <fieldset className="mt-4 grid gap-3 rounded-md border border-slate-200 bg-slate-50 p-3 sm:grid-cols-2 lg:grid-cols-5">
            <legend className="px-1 text-sm font-semibold text-slate-800">
              筛选与排序
            </legend>
            <Field label="搜索线索">
              <Input
                aria-label="搜索线索"
                value={queueFilters.query}
                maxLength={200}
                placeholder="内容、来源、地点或状态"
                onChange={(event) =>
                  setQueueFilters((current) => ({
                    ...current,
                    query: event.target.value,
                    page: 1,
                  }))
                }
                fullWidth
              />
            </Field>
            <Field label="审核状态">
              <select
                aria-label="审核状态筛选"
                className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                value={queueFilters.status}
                onChange={(event) =>
                  setQueueFilters((current) => ({
                    ...current,
                    status: event.target.value as ClueStatus | "",
                    page: 1,
                  }))
                }
              >
                <option value="">全部状态</option>
                {Object.entries(statusLabels)
                  .filter(
                    ([status]) =>
                      !["active", "resolved", "closed"].includes(status),
                  )
                  .map(([status, label]) => (
                    <option key={status} value={status}>
                      {label}
                    </option>
                  ))}
              </select>
            </Field>
            <Field label="来源类型">
              <select
                aria-label="来源类型筛选"
                className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                value={queueFilters.sourceType}
                onChange={(event) =>
                  setQueueFilters((current) => ({
                    ...current,
                    sourceType: event.target.value as ClueSourceType | "",
                    page: 1,
                  }))
                }
              >
                <option value="">全部来源</option>
                <option value="manual_report">人工上报</option>
                <option value="field_report">现场反馈</option>
                <option value="chat_draft">聊天整理草稿</option>
                <option value="ai_draft">智能整理草稿</option>
              </select>
            </Field>
            <Field label="时间字段">
              <select
                aria-label="时间字段排序"
                className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                value={queueFilters.sort}
                onChange={(event) =>
                  setQueueFilters((current) => ({
                    ...current,
                    sort: event.target.value as ClueQueueFilters["sort"],
                    page: 1,
                  }))
                }
              >
                <option value="created_at">上报时间</option>
                <option value="occurred_at">发生时间</option>
              </select>
            </Field>
            <Field label="排序方向">
              <select
                aria-label="排序方向"
                className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                value={queueFilters.order}
                onChange={(event) =>
                  setQueueFilters((current) => ({
                    ...current,
                    order: event.target.value as ClueQueueFilters["order"],
                    page: 1,
                  }))
                }
              >
                <option value="desc">最新优先</option>
                <option value="asc">最早优先</option>
              </select>
            </Field>
          </fieldset>
        )}

        {isCommander && queueError && (
          <div className="mt-4">
            <ErrorState
              message={queueError}
              onRetry={() => void loadClueQueue()}
            />
          </div>
        )}

        <div className="mt-4 divide-y divide-slate-100 border-y border-slate-200">
          {isQueueLoading ? (
            <LoadingState label="正在加载审核队列" />
          ) : visibleClues.length === 0 ? (
            <div className="flex min-h-28 items-center justify-center text-sm text-slate-500">
              暂无可见线索
            </div>
          ) : (
            visibleClues.map((clue) => (
              <article key={clue.id} className="py-4">
                <div className="flex flex-wrap items-center gap-2">
                  <Chip size="sm" variant="soft">
                    <Chip.Label>
                      {statusLabels[clue.status] ?? clue.status}
                    </Chip.Label>
                  </Chip>
                  <span className="text-xs text-slate-500">{clue.source}</span>
                  {clue.source_type !== "manual_report" && (
                    <span className="text-xs font-medium text-amber-700">
                      {clue.source_type === "ai_draft"
                        ? "智能整理草稿"
                        : clue.source_type === "chat_draft"
                          ? "聊天整理草稿"
                          : "现场反馈"}
                    </span>
                  )}
                  {clue.is_own_submission && (
                    <span className="text-xs font-medium text-brand-700">
                      本人提交
                    </span>
                  )}
                  <span className="ml-auto text-xs text-slate-500">
                    事件：{formatDate(clue.occurred_at) ?? "未提供"} · 上报：
                    {formatDate(clue.reported_at)}
                  </span>
                </div>
                <p className="m-0 mt-2 whitespace-pre-wrap text-sm leading-6 text-slate-700">
                  {clue.content}
                </p>
                {clue.raw_record_reference && (
                  <p className="m-0 mt-1 text-xs text-slate-500">
                    受控原始记录：{clue.raw_record_reference}
                  </p>
                )}
                {clue.location_text && (
                  <p className="m-0 mt-1 text-xs text-slate-500">
                    地点：{clue.location_text}
                    {clue.location_precision
                      ? `（${clue.location_precision === "exact" ? "精确" : clue.location_precision === "approximate" ? "约略" : "精度未知"}）`
                      : ""}
                  </p>
                )}
                {(clue.next_action ||
                  clue.linked_task_reference ||
                  clue.review_reason ||
                  clue.confirmed_at) && (
                  <div className="mt-2 rounded bg-slate-50 px-2 py-1.5 text-xs leading-5 text-slate-600">
                    {clue.review_reason && (
                      <div>审核理由：{clue.review_reason}</div>
                    )}
                    {clue.confirmed_at && (
                      <div>确认时间：{formatDate(clue.confirmed_at)}</div>
                    )}
                    {clue.next_action && <div>下一步：{clue.next_action}</div>}
                    {clue.linked_task_reference && (
                      <div>关联任务：{clue.linked_task_reference}</div>
                    )}
                  </div>
                )}
                {isCommander && clue.status === "pending_review" && (
                  <div className="mt-3 grid gap-3 rounded-md border border-amber-200 bg-amber-50 p-3">
                    <Field
                      label="审核理由（每次确认、排除、降级或合并必填）"
                      required
                    >
                      <TextArea
                        aria-label="审核理由"
                        value={reviewDrafts[clue.id]?.reason ?? ""}
                        maxLength={1000}
                        rows={2}
                        onChange={(event) =>
                          setReviewDrafts((current) => ({
                            ...current,
                            [clue.id]: {
                              reason: event.target.value,
                              relatedClueId:
                                current[clue.id]?.relatedClueId ?? "",
                              relatedClueQuery:
                                current[clue.id]?.relatedClueQuery ?? "",
                            },
                          }))
                        }
                        fullWidth
                        required
                      />
                    </Field>
                    <Field label="关联线索（重复或冲突时必选）">
                      <RelatedCluePicker
                        clueId={clue.id}
                        candidates={detail.clues}
                        selectedId={reviewDrafts[clue.id]?.relatedClueId ?? ""}
                        query={reviewDrafts[clue.id]?.relatedClueQuery ?? ""}
                        onChange={(relatedClueId, relatedClueQuery) =>
                          setReviewDrafts((current) => ({
                            ...current,
                            [clue.id]: {
                              reason: current[clue.id]?.reason ?? "",
                              relatedClueId,
                              relatedClueQuery,
                            },
                          }))
                        }
                      />
                    </Field>
                    <p className="m-0 text-xs leading-5 text-amber-900">
                      审核只会由指挥端提交。每条线索各自保存审核草稿，成功审核后会清空该条草稿。
                    </p>
                    <div className="flex flex-wrap gap-2">
                      <ReviewButton
                        label="确认"
                        status="confirmed"
                        clueId={clue.id}
                        reason={reviewDrafts[clue.id]?.reason ?? ""}
                        relatedClueId={
                          reviewDrafts[clue.id]?.relatedClueId ?? ""
                        }
                        busy={busy}
                        run={run}
                        onReviewed={() =>
                          setReviewDrafts((current) => {
                            const next = { ...current };
                            delete next[clue.id];
                            return next;
                          })
                        }
                      />
                      <ReviewButton
                        label="待核实"
                        status="needs_verification"
                        clueId={clue.id}
                        reason={reviewDrafts[clue.id]?.reason ?? ""}
                        relatedClueId={
                          reviewDrafts[clue.id]?.relatedClueId ?? ""
                        }
                        busy={busy}
                        run={run}
                        onReviewed={() =>
                          setReviewDrafts((current) => {
                            const next = { ...current };
                            delete next[clue.id];
                            return next;
                          })
                        }
                      />
                      <ReviewButton
                        label="信息不足"
                        status="insufficient_information"
                        clueId={clue.id}
                        reason={reviewDrafts[clue.id]?.reason ?? ""}
                        relatedClueId={
                          reviewDrafts[clue.id]?.relatedClueId ?? ""
                        }
                        busy={busy}
                        run={run}
                        onReviewed={() =>
                          setReviewDrafts((current) => {
                            const next = { ...current };
                            delete next[clue.id];
                            return next;
                          })
                        }
                      />
                      <ReviewButton
                        label="排除"
                        status="rejected"
                        clueId={clue.id}
                        reason={reviewDrafts[clue.id]?.reason ?? ""}
                        relatedClueId={
                          reviewDrafts[clue.id]?.relatedClueId ?? ""
                        }
                        busy={busy}
                        run={run}
                        onReviewed={() =>
                          setReviewDrafts((current) => {
                            const next = { ...current };
                            delete next[clue.id];
                            return next;
                          })
                        }
                      />
                      <ReviewButton
                        label="重复"
                        status="duplicate"
                        clueId={clue.id}
                        reason={reviewDrafts[clue.id]?.reason ?? ""}
                        relatedClueId={
                          reviewDrafts[clue.id]?.relatedClueId ?? ""
                        }
                        busy={busy}
                        run={run}
                        onReviewed={() =>
                          setReviewDrafts((current) => {
                            const next = { ...current };
                            delete next[clue.id];
                            return next;
                          })
                        }
                      />
                      <ReviewButton
                        label="冲突"
                        status="conflicting"
                        clueId={clue.id}
                        reason={reviewDrafts[clue.id]?.reason ?? ""}
                        relatedClueId={
                          reviewDrafts[clue.id]?.relatedClueId ?? ""
                        }
                        busy={busy}
                        run={run}
                        onReviewed={() =>
                          setReviewDrafts((current) => {
                            const next = { ...current };
                            delete next[clue.id];
                            return next;
                          })
                        }
                      />
                    </div>
                  </div>
                )}
              </article>
            ))
          )}
        </div>

        {isCommander && clueQueue.total > clueQueue.pageSize && (
          <nav
            className="mt-4 flex items-center justify-between gap-3"
            aria-label="线索队列分页"
          >
            <span className="text-sm text-slate-600">
              第 {clueQueue.page} 页，共{" "}
              {Math.ceil(clueQueue.total / clueQueue.pageSize)} 页
            </span>
            <div className="flex gap-2">
              <Button
                size="sm"
                variant="ghost"
                isDisabled={isQueueLoading || clueQueue.page <= 1}
                onPress={() =>
                  setQueueFilters((current) => ({
                    ...current,
                    page: current.page - 1,
                  }))
                }
              >
                上一页
              </Button>
              <Button
                size="sm"
                variant="ghost"
                isDisabled={
                  isQueueLoading ||
                  clueQueue.page * clueQueue.pageSize >= clueQueue.total
                }
                onPress={() =>
                  setQueueFilters((current) => ({
                    ...current,
                    page: current.page + 1,
                  }))
                }
              >
                下一页
              </Button>
            </div>
          </nav>
        )}

        {detail.status === "active" && (
          <details className="mt-5 rounded-md border border-slate-200 bg-slate-50">
            <summary className="cursor-pointer px-3 py-3 text-sm font-semibold text-slate-900">
              {isCommander ? "提交补充线索" : "提交一条新线索"}
            </summary>
            <form
              className="grid gap-3 border-t border-slate-200 p-3 sm:grid-cols-[minmax(0,1fr)_220px] sm:p-4"
            onSubmit={(event) => {
              event.preventDefault();
              if (!token) return;
              const content = nullable(clueContent);
              if (!content) {
                setError("请填写线索内容后再提交。");
                return;
              }
              const location = nullable(clueLocation);
              if (clueLocationPrecision && !location) {
                setError("选择地点精度时，请同时填写地点。");
                return;
              }
              void run(
                "clue",
                () =>
                  createClue(token, detail.id, {
                    source: detail.access_role,
                    content,
                    source_type: clueSourceType,
                    raw_record_reference: nullable(clueRawReference),
                    occurred_at: toIsoOrNull(clueOccurredAt),
                    location_text: location,
                    location_precision: clueLocationPrecision || null,
                    next_action: nullable(clueNextAction),
                    attachment_ids: linkedAttachmentIds,
                  }),
                "线索已提交并进入人工审核",
              ).then((succeeded) => {
                if (succeeded) {
                  setClueContent("");
                  setClueLocation("");
                  setClueOccurredAt("");
                  setClueSourceType("manual_report");
                  setClueRawReference("");
                  setClueLocationPrecision("");
                  setClueNextAction("");
                  setLinkedAttachmentIds([]);
                }
              });
            }}
          >
            <Field label="线索内容" required>
              <TextArea
                value={clueContent}
                maxLength={4000}
                onChange={(event) => setClueContent(event.target.value)}
                rows={3}
                fullWidth
                required
              />
            </Field>
            <div className="space-y-3">
              <Field label="来源类型">
                <select
                  className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                  value={clueSourceType}
                  onChange={(event) =>
                    setClueSourceType(
                      event.target.value as PublicClueSourceType,
                    )
                  }
                >
                  <option value="manual_report">人工上报</option>
                  <option value="field_report">现场反馈</option>
                </select>
              </Field>
              <Field label="发生时间">
                <Input
                  type="datetime-local"
                  value={clueOccurredAt}
                  onChange={(event) => setClueOccurredAt(event.target.value)}
                  fullWidth
                />
              </Field>
              <Field label="地点">
                <Input
                  value={clueLocation}
                  onChange={(event) => setClueLocation(event.target.value)}
                  fullWidth
                />
              </Field>
              <Field label="地点精度">
                <select
                  className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                  value={clueLocationPrecision}
                  onChange={(event) =>
                    setClueLocationPrecision(
                      event.target.value as LocationPrecision | "",
                    )
                  }
                >
                  <option value="">未提供</option>
                  <option value="exact">精确</option>
                  <option value="approximate">约略</option>
                  <option value="unknown">未知</option>
                </select>
              </Field>
              <Field label="受控原始记录引用">
                <Input
                  value={clueRawReference}
                  maxLength={500}
                  onChange={(event) => setClueRawReference(event.target.value)}
                  fullWidth
                />
              </Field>
              <Field label="下一步动作">
                <Input
                  value={clueNextAction}
                  maxLength={500}
                  onChange={(event) => setClueNextAction(event.target.value)}
                  fullWidth
                />
              </Field>
              {detail.attachments.some(
                (attachment) => attachment.is_own_submission,
              ) && (
                <Field label="关联本人附件">
                  <select
                    multiple
                    className="min-h-20 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                    value={linkedAttachmentIds}
                    onChange={(event) =>
                      setLinkedAttachmentIds(
                        Array.from(
                          event.target.selectedOptions,
                          (option) => option.value,
                        ),
                      )
                    }
                  >
                    {detail.attachments
                      .filter((attachment) => attachment.is_own_submission)
                      .map((attachment) => (
                        <option key={attachment.id} value={attachment.id}>
                          {attachment.original_filename}
                        </option>
                      ))}
                  </select>
                </Field>
              )}
              <Button
                type="submit"
                variant="primary"
                fullWidth
                isDisabled={busy === "clue"}
              >
                <Send size={16} />
                提交线索
              </Button>
            </div>
            </form>
          </details>
        )}
      </section>

      <section
        className={`grid gap-6 border-t border-slate-200 bg-slate-50 px-5 py-5 sm:px-6 ${canSubmitPlace ? "lg:grid-cols-2" : ""}`}
        aria-label="补充地点和图片"
      >
        {canSubmitPlace && (
          <div>
            <div className="flex items-center justify-between gap-3">
              <h3 className="m-0 text-base font-bold text-slate-950">
                补充地点
              </h3>
              <span className="text-xs text-slate-500">提交后待人工审核</span>
            </div>
            <div className="mt-3 divide-y divide-slate-200 rounded-md border border-slate-200 bg-white">
              {detail.places.length === 0 ? (
                <p className="m-0 px-3 py-3 text-xs text-slate-500">
                  暂无可查看的补充地点
                </p>
              ) : (
                detail.places.map((item) => (
                  <div key={item.id} className="px-3 py-3 text-sm">
                    <strong className="text-slate-900">{item.name}</strong>
                    <span className="ml-2 text-xs text-slate-500">
                      {item.review_status === "pending_review"
                        ? "待人工审核"
                        : item.review_status}
                    </span>
                    <p className="m-0 mt-1 text-xs text-slate-600">
                      {item.address}
                    </p>
                  </div>
                ))
              )}
            </div>
            {detail.status !== "closed" && (
              <form
                className="mt-3 grid gap-3"
                onSubmit={(event) => {
                  event.preventDefault();
                  if (
                    !token ||
                    !place.name.trim() ||
                    !place.address.trim() ||
                    !place.place_type
                  ) {
                    setError("请填写地点名称、类型和文字地址后再提交。");
                    return;
                  }
                  if (
                    (place.longitude === null) !==
                    (place.latitude === null)
                  ) {
                    setError("经度和纬度必须同时填写或同时留空。");
                    return;
                  }
                  void run(
                    "place",
                    () =>
                      createCasePlace(token, detail.id, {
                        ...place,
                        name: place.name.trim(),
                        address: place.address.trim(),
                      }),
                    "地点已提交，正在等待人工审核",
                  ).then((ok) => {
                    if (ok)
                      setPlace({
                        name: "",
                        place_type: placeTypes[0] ?? "",
                        address: "",
                        longitude: null,
                        latitude: null,
                        visibility: "confirmed",
                      });
                  });
                }}
              >
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label="地点名称" required>
                    <Input
                      value={place.name}
                      maxLength={120}
                      onChange={(event) =>
                        setPlace({ ...place, name: event.target.value })
                      }
                      fullWidth
                      required
                    />
                  </Field>
                  <Field label="类型">
                    <select
                      className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                      value={place.place_type}
                      onChange={(event) =>
                        setPlace({
                          ...place,
                          place_type: event.target.value as PlaceType,
                        })
                      }
                    >
                      {placeTypes.map((type) => (
                        <option key={type} value={type}>
                          {placeTypeLabels[type] ?? type}
                        </option>
                      ))}
                    </select>
                  </Field>
                </div>
                <Field label="文字地址" required>
                  <Input
                    value={place.address}
                    maxLength={500}
                    onChange={(event) =>
                      setPlace({ ...place, address: event.target.value })
                    }
                    fullWidth
                    required
                  />
                </Field>
                <div className="grid gap-3 sm:grid-cols-3">
                  <Field label="经度（可选）">
                    <Input
                      type="number"
                      min={-180}
                      max={180}
                      value={place.longitude ?? ""}
                      onChange={(event) =>
                        setPlace({
                          ...place,
                          longitude:
                            event.target.value === ""
                              ? null
                              : Number(event.target.value),
                        })
                      }
                      fullWidth
                    />
                  </Field>
                  <Field label="纬度（可选）">
                    <Input
                      type="number"
                      min={-90}
                      max={90}
                      value={place.latitude ?? ""}
                      onChange={(event) =>
                        setPlace({
                          ...place,
                          latitude:
                            event.target.value === ""
                              ? null
                              : Number(event.target.value),
                        })
                      }
                      fullWidth
                    />
                  </Field>
                  <Field label="可见级别">
                    <select
                      className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
                      value={place.visibility}
                      onChange={(event) =>
                        setPlace({
                          ...place,
                          visibility: event.target.value as PlaceVisibility,
                        })
                      }
                    >
                      <option value="confirmed">已确认范围</option>
                      <option value="internal">仅内部</option>
                      <option value="public">公开范围</option>
                    </select>
                  </Field>
                </div>
                <Button
                  type="submit"
                  variant="secondary"
                  isDisabled={busy === "place"}
                >
                  提交地点
                </Button>
              </form>
            )}
          </div>
        )}
        <div>
          <div className="flex items-center justify-between gap-3">
            <h3 className="m-0 text-base font-bold text-slate-950">补充图片</h3>
            <span className="text-xs text-slate-500">
              仅 JPEG/PNG，最大{" "}
              {formatBytes(resourceConfiguration.attachment_max_image_bytes)}
            </span>
          </div>
          <div className="mt-3 divide-y divide-slate-200 rounded-md border border-slate-200 bg-white">
            {detail.attachments.length === 0 ? (
              <p className="m-0 px-3 py-3 text-xs text-slate-500">
                暂无可查看的图片
              </p>
            ) : (
              detail.attachments.map((item) => (
                <div
                  key={item.id}
                  className="flex items-center justify-between gap-2 px-3 py-3 text-sm"
                >
                  <span className="truncate text-slate-800">
                    {item.original_filename}
                  </span>
                  <span className="shrink-0 text-xs text-slate-500">
                    {item.review_status === "pending_review"
                      ? "待人工审核"
                      : item.review_status}
                  </span>
                </div>
              ))
            )}
          </div>
          {detail.status !== "closed" && (
            <form
              className="mt-3 grid gap-3"
              onSubmit={(event) => {
                event.preventDefault();
                if (!token || !attachment) {
                  setError("请选择一张图片后再提交。");
                  return;
                }
                void run(
                  "attachment",
                  () =>
                    uploadCaseAttachment(
                      token,
                      detail.id,
                      attachment,
                      resourceConfiguration.attachment_max_image_bytes,
                    ),
                  "图片已提交，正在等待人工审核",
                ).then((ok) => {
                  if (ok) setAttachment(null);
                });
              }}
            >
              <input
                key={
                  attachment
                    ? `${attachment.name}-${attachment.lastModified}`
                    : "no-file"
                }
                type="file"
                accept="image/jpeg,image/png"
                onChange={(event) =>
                  setAttachment(event.target.files?.[0] ?? null)
                }
                className="block w-full text-sm text-slate-700"
              />
              <Button
                type="submit"
                variant="secondary"
                isDisabled={busy === "attachment"}
              >
                上传图片
              </Button>
              <p className="m-0 text-xs leading-5 text-slate-500">
                上传会由服务端重新编码并移除非必要的 EXIF/GPS
                元数据；失败时不会显示为上传成功。
              </p>
            </form>
          )}
        </div>
      </section>
    </div>
  );
}

function RoleActionPanel({
  accessRole,
  caseStatus,
  pendingCount,
  hasProfileGaps,
}: {
  accessRole: CaseRole;
  caseStatus: CaseStatus;
  pendingCount: number;
  hasProfileGaps: boolean;
}) {
  const isCommander = accessRole === "commander";
  const isFamily = accessRole === "family";
  const isActive = caseStatus === "active";
  const heading = isCommander
    ? "指挥工作台"
    : isFamily
      ? "当前可以做什么"
      : "协作提示";
  const description = isCommander
    ? pendingCount > 0
      ? `有 ${pendingCount} 条线索等待人工审核，请先完成审核并记录理由。`
      : "当前没有待审核线索；可在任务看板查看或创建受控任务。"
    : isFamily
      ? !isActive
        ? caseStatus === "resolved"
          ? "案件已找到，不能再提交补充信息。"
          : "案件已关闭，不能再提交补充信息。"
        : hasProfileGaps
        ? "请先补充关键人物资料，再提交新的线索、地点或图片。"
        : "可补充线索、地点或图片；提交的信息会先进入人工审核。"
      : "请查看已分配任务与经服务端裁剪后的案件信息。";
  const action = isCommander
    ? "前往任务和审核区"
    : isFamily
      ? isActive && hasProfileGaps
        ? "补充人物资料"
        : isActive
          ? "提交一条新线索"
          : "查看案件资料"
      : "查看已分配任务";
  const target = isCommander
    ? "#task-board"
    : isFamily
      ? isActive && hasProfileGaps
        ? "#case-profile-editor"
        : isActive
          ? "#case-clues"
          : "#case-profile"
      : "#task-board";

  return (
    <section
      id="case-actions"
      className="border-b border-emerald-200 bg-emerald-50/70 px-5 py-5 sm:px-6"
      aria-labelledby="role-actions-title"
    >
      <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
        <div className="max-w-2xl">
          <p className="m-0 text-xs font-semibold text-emerald-800">
            {isCommander ? "先处理待办，再查看资料" : "根据当前案件状态继续"}
          </p>
          <h3 id="role-actions-title" className="mb-0 mt-1 text-lg font-bold text-slate-950">
            {heading}
          </h3>
          <p className="mb-0 mt-1 text-sm leading-6 text-slate-700">
            {description}
          </p>
        </div>
        <a
          className="inline-flex min-h-10 items-center justify-center rounded-md bg-brand-700 px-4 text-sm font-semibold text-white no-underline transition-colors hover:bg-brand-800 focus:outline-none focus:ring-2 focus:ring-brand-700 focus:ring-offset-2"
          href={target}
          aria-label={`${action}（主操作）`}
        >
          {action}
        </a>
      </div>
    </section>
  );
}

const taskStatusLabels: Record<string, string> = {
  pending_claim: "待志愿者申请",
  assigned: "待领取",
  accepted: "已接受",
  active: "进行中",
  blocked: "受阻",
  completed: "已完成",
  cancelled: "已取消",
};
const emptyTask = (): CreateTaskPayload => ({
  source_clue_id: "",
  volunteer_user_id: undefined,
  title: "",
  objective: "",
  area_text: "",
  latitude: null,
  longitude: null,
  due_at: "",
  background: "",
  risk_level: "low",
  risk_notes: "",
  safety_briefing: "",
  expected_feedback: "",
});

function TaskBoard({
  detail,
  token,
}: {
  detail: CaseDetail;
  token: string | null;
}) {
  const [tasks, setTasks] = useState<CaseTask[]>([]);
  const [members, setMembers] = useState<CaseMember[]>([]);
  const [draft, setDraft] = useState<CreateTaskPayload>(emptyTask);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState("");
  const [feedback, setFeedback] = useState<Record<string, string>>({});
  const isCommander = detail.access_role === "commander";
  const load = useCallback(async () => {
    if (!token) return;
    setLoading(true);
    setError("");
    try {
      const [page, nextMembers] = await Promise.all([
        listCaseTasks(token, detail.id),
        isCommander ? listCaseMembers(token, detail.id) : Promise.resolve([]),
      ]);
      setTasks(page.items);
      setMembers(nextMembers);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setLoading(false);
    }
  }, [detail.id, isCommander, token]);
  useEffect(() => {
    setDraft(emptyTask());
    void load();
  }, [detail.id, load]);
  const volunteers = members.filter(
    (member) => member.case_role === "volunteer",
  );
  const confirmedClues = detail.clues.filter(
    (clue) => clue.status === "confirmed",
  );
  const createsOpenTask = !draft.volunteer_user_id;
  async function changeStatus(
    taskId: string,
    status: "accepted" | "active" | "blocked" | "completed" | "cancelled",
  ) {
    if (!token) return;
    setBusy(taskId);
    setError("");
    try {
      await updateTaskStatus(token, taskId, status);
      await load();
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy("");
    }
  }
  async function applyToTask(taskId: string) {
    if (!token) return;
    setBusy(taskId);
    setError("");
    setNotice("");
    try {
      await applyForTask(token, taskId);
      setNotice("申请已提交，等待指挥审核。");
      await load();
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy("");
    }
  }
  async function sendFeedback(taskId: string) {
    const content = feedback[taskId]?.trim();
    if (!token || !content) return;
    setBusy(`feedback-${taskId}`);
    setError("");
    try {
      await submitTaskFeedback(token, taskId, {
        content,
        occurred_at: null,
        location_text: null,
        location_precision: null,
      });
      setFeedback((current) => ({ ...current, [taskId]: "" }));
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy("");
    }
  }
  return (
    <section
      id="task-board"
      className="border-b border-slate-200 px-5 py-5 sm:px-6"
      aria-label="任务看板"
    >
      <span id="case-tasks" aria-hidden="true" />
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="m-0 text-base font-bold text-slate-950">任务看板</h3>
          <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
            任务只由指挥人工创建和分配；状态、人员与失败结果均以服务端返回为准。
          </p>
        </div>
        <Button
          size="sm"
          variant="ghost"
          isDisabled={loading}
          onPress={() => void load()}
        >
          <RefreshCw size={16} />
          刷新任务
        </Button>
      </div>
      {error && (
        <div className="mt-3">
          <Message tone="error">{error}</Message>
        </div>
      )}
      {notice && (
        <div className="mt-3">
          <Message tone="success">{notice}</Message>
        </div>
      )}
      {loading ? (
        <p className="mb-0 mt-3 text-sm text-slate-500">正在加载任务…</p>
      ) : tasks.length === 0 ? (
        <p className="mb-0 mt-3 text-sm text-slate-500">
          当前没有你可查看的任务。
        </p>
      ) : (
        <div className="mt-3 grid gap-3 lg:grid-cols-2">
          {tasks.map((task) => (
            <article
              key={task.id}
              className="rounded-md border border-slate-200 bg-white p-3"
            >
              <div className="flex flex-wrap items-center gap-2">
                <Chip size="sm" variant="soft">
                  <Chip.Label>
                    {taskStatusLabels[task.status] ?? task.status}
                  </Chip.Label>
                </Chip>
                <Chip size="sm" variant="soft">
                  <Chip.Label>{task.risk_level} 风险</Chip.Label>
                </Chip>
              </div>
              <h4 className="mb-0 mt-3 text-sm font-semibold text-slate-950">
                {task.title}
              </h4>
              <p className="mb-0 mt-1 text-sm text-slate-700">
                {task.objective}
              </p>
              <p className="mb-0 mt-2 text-xs text-slate-500">
                区域：{task.area_text} · 截止：
                {formatDate(task.due_at) ?? "待补充"}
              </p>
              <p className="mb-0 mt-1 text-xs text-slate-600">
                安全提示：{task.safety_briefing}
              </p>
              {detail.access_role === "volunteer" && (
                <>
                  <div className="mt-3 flex flex-wrap gap-2">
                    {task.status === "pending_claim" && (
                      <Button
                        size="sm"
                        variant="secondary"
                        isDisabled={busy === task.id}
                        onPress={() => void applyToTask(task.id)}
                      >
                        申请协作
                      </Button>
                    )}
                    {task.status === "assigned" && (
                      <Button
                        size="sm"
                        variant="secondary"
                        isDisabled={busy === task.id}
                        onPress={() => void changeStatus(task.id, "accepted")}
                      >
                        接受
                      </Button>
                    )}
                    {task.status === "accepted" && (
                      <Button
                        size="sm"
                        variant="secondary"
                        isDisabled={busy === task.id}
                        onPress={() => void changeStatus(task.id, "active")}
                      >
                        开始执行
                      </Button>
                    )}
                    {task.status === "active" && (
                      <>
                        <Button
                          size="sm"
                          variant="secondary"
                          isDisabled={busy === task.id}
                          onPress={() => void changeStatus(task.id, "blocked")}
                        >
                          标记受阻
                        </Button>
                        <Button
                          size="sm"
                          variant="primary"
                          isDisabled={busy === task.id}
                          onPress={() =>
                            void changeStatus(task.id, "completed")
                          }
                        >
                          完成
                        </Button>
                      </>
                    )}
                    {task.status === "blocked" && (
                      <Button
                        size="sm"
                        variant="secondary"
                        isDisabled={busy === task.id}
                        onPress={() => void changeStatus(task.id, "active")}
                      >
                        继续执行
                      </Button>
                    )}
                  </div>
                  {task.status === "active" && (
                    <div className="mt-3">
                      <TextArea
                        aria-label={`${task.title} 任务反馈`}
                        value={feedback[task.id] ?? ""}
                        maxLength={4000}
                        rows={2}
                        placeholder="提交现场反馈；将进入人工审核，不会直接成为确认事实"
                        onChange={(event) =>
                          setFeedback((current) => ({
                            ...current,
                            [task.id]: event.target.value,
                          }))
                        }
                        fullWidth
                      />
                      <div className="mt-2 flex justify-end">
                        <Button
                          size="sm"
                          variant="secondary"
                          isDisabled={
                            !feedback[task.id]?.trim() ||
                            busy === `feedback-${task.id}`
                          }
                          onPress={() => void sendFeedback(task.id)}
                        >
                          提交待审核反馈
                        </Button>
                      </div>
                    </div>
                  )}
                </>
              )}
              {isCommander &&
                !["completed", "cancelled"].includes(task.status) && (
                  <div className="mt-3">
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={busy === task.id}
                      onPress={() => void changeStatus(task.id, "cancelled")}
                    >
                      取消任务
                    </Button>
                  </div>
                )}
            </article>
          ))}
        </div>
      )}
      {isCommander && detail.status !== "closed" && (
        <form
          className="mt-5 grid gap-3 border-t border-slate-200 pt-5 lg:grid-cols-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (!token) return;
            setBusy("create");
            setError("");
            createCaseTask(token, detail.id, draft)
              .then(async () => {
                setDraft(emptyTask());
                await load();
              })
              .catch((cause) => setError(messageFrom(cause)))
              .finally(() => setBusy(""));
          }}
        >
          <h4 className="m-0 text-sm font-semibold text-slate-950 lg:col-span-2">
            {createsOpenTask ? "创建开放任务" : "人工创建并分配任务"}
          </h4>
          <Field label="关联的已确认线索" required>
            <select
              required
              className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
              value={draft.source_clue_id}
              onChange={(event) =>
                setDraft({ ...draft, source_clue_id: event.target.value })
              }
            >
              <option value="">请选择</option>
              {confirmedClues.map((clue) => (
                <option key={clue.id} value={clue.id}>
                  {clue.content.slice(0, 80)}
                </option>
              ))}
            </select>
          </Field>
          <Field label="志愿者">
            <select
              className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
              value={draft.volunteer_user_id ?? ""}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  volunteer_user_id: event.target.value || undefined,
                })
              }
            >
              <option value="">开放任务：等待志愿者申请</option>
              {volunteers.map((member) => (
                <option key={member.user_id} value={member.user_id}>
                  {member.display_name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="任务标题" required>
            <Input
              required
              value={draft.title}
              maxLength={200}
              onChange={(event) =>
                setDraft({ ...draft, title: event.target.value })
              }
              fullWidth
            />
          </Field>
          <Field label="任务区域" required>
            <Input
              required
              value={draft.area_text}
              maxLength={500}
              onChange={(event) =>
                setDraft({ ...draft, area_text: event.target.value })
              }
              fullWidth
            />
          </Field>
          <Field label="目标" required>
            <TextArea
              required
              value={draft.objective}
              maxLength={4000}
              rows={2}
              onChange={(event) =>
                setDraft({ ...draft, objective: event.target.value })
              }
              fullWidth
            />
          </Field>
          <Field label="截止时间" required>
            <Input
              required
              type="datetime-local"
              value={toDateTimeLocal(draft.due_at)}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  due_at: event.target.value
                    ? new Date(event.target.value).toISOString()
                    : "",
                })
              }
              fullWidth
            />
          </Field>
          <Field label="背景说明" required>
            <TextArea
              required
              value={draft.background}
              maxLength={10000}
              rows={2}
              onChange={(event) =>
                setDraft({ ...draft, background: event.target.value })
              }
              fullWidth
            />
          </Field>
          <Field label="风险说明与安全提示" required>
            <TextArea
              required
              value={draft.risk_notes}
              maxLength={4000}
              rows={2}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  risk_notes: event.target.value,
                  safety_briefing: draft.safety_briefing || event.target.value,
                })
              }
              fullWidth
            />
          </Field>
          <Field label="预期反馈" required>
            <TextArea
              required
              value={draft.expected_feedback}
              maxLength={4000}
              rows={2}
              onChange={(event) =>
                setDraft({ ...draft, expected_feedback: event.target.value })
              }
              fullWidth
            />
          </Field>
          <div className="flex items-end">
            <Button
              type="submit"
              size="sm"
              variant="primary"
              isDisabled={busy === "create" || confirmedClues.length === 0}
            >
              {createsOpenTask ? "创建并等待志愿者申请" : "创建并分配"}
            </Button>
          </div>
        </form>
      )}
    </section>
  );
}

const mapObjectLabels: Record<CaseMapItem["object_type"], string> = {
  last_seen: "最后出现信息",
  place: "补充地点",
  clue: "已确认线索",
  task: "任务区域",
};

const precisionLabels: Record<CaseMapItem["location_precision"], string> = {
  exact: "精确位置",
  approximate: "模糊地点",
  unknown: "仅文字地点",
};

function CaseSituationPanel({
  detail,
  token,
}: {
  detail: CaseDetail;
  token: string | null;
}) {
  const [mapView, setMapView] = useState<CaseMapView | null>(null);
  const [filter, setFilter] = useState<CaseMapItem["object_type"] | "all">(
    "all",
  );
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState("");
  const requestVersion = useRef(0);

  const loadMapView = useCallback(async () => {
    const version = requestVersion.current + 1;
    requestVersion.current = version;
    if (!token) {
      setMapView(null);
      setError("");
      setIsLoading(false);
      return;
    }
    setIsLoading(true);
    setError("");
    try {
      const response = await getCaseMapView(token, detail.id);
      if (version !== requestVersion.current) return;
      setMapView(response);
    } catch (cause) {
      if (version !== requestVersion.current) return;
      setMapView(null);
      setError(messageFrom(cause));
    } finally {
      if (version === requestVersion.current) setIsLoading(false);
    }
  }, [detail.id, token]);

  useEffect(() => {
    setFilter("all");
    void loadMapView();
    return () => {
      requestVersion.current += 1;
    };
  }, [detail.id, loadMapView]);

  const items = useMemo(
    () =>
      mapView?.items.filter(
        (item) => filter === "all" || item.object_type === filter,
      ) ?? [],
    [filter, mapView],
  );
  const coordinateCount = items.filter(
    (item) => item.longitude !== null && item.latitude !== null,
  ).length;

  return (
    <section
      className="border-b border-slate-200 bg-slate-50 px-5 py-5 sm:px-6"
      aria-label="地图态势与文字降级"
    >
      <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
        <div>
          <h3 className="m-0 text-base font-bold text-slate-950">
            地图态势与文字降级
          </h3>
          <p className="mb-0 mt-1 max-w-3xl text-xs leading-5 text-slate-600">
            仅展示服务端按当前案件角色返回的地点、已确认线索和任务区域。当前首轮使用可审查的文字态势；地图供应商不可用时，以下地点、精度和状态仍可使用。
          </p>
        </div>
        <Button
          size="sm"
          variant="ghost"
          isDisabled={!token || isLoading}
          onPress={() => void loadMapView()}
        >
          <RefreshCw size={16} />
          刷新态势
        </Button>
      </div>

      <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-center">
        <label
          className="text-sm font-medium text-slate-700"
          htmlFor="map-object-filter"
        >
          对象类型
        </label>
        <select
          id="map-object-filter"
          aria-label="态势对象筛选"
          className="min-h-10 rounded-md border border-slate-300 bg-white px-3 text-sm"
          value={filter}
          onChange={(event) =>
            setFilter(event.target.value as CaseMapItem["object_type"] | "all")
          }
        >
          <option value="all">全部对象</option>
          {Object.entries(mapObjectLabels).map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
        {mapView && (
          <span className="text-xs text-slate-500">
            {items.length} 项可见，其中 {coordinateCount} 项具备已授权坐标。
          </span>
        )}
      </div>

      {isLoading && (
        <p className="mb-0 mt-4 text-sm text-slate-500" role="status">
          正在加载角色化态势…
        </p>
      )}
      {error && (
        <div className="mt-4">
          <Message tone="error">
            地图态势暂不可用：{error}
            。请使用案件资料、任务说明和人工联系路径继续工作。
          </Message>
        </div>
      )}
      {!isLoading && !error && mapView && items.length === 0 && (
        <p className="mb-0 mt-4 text-sm text-slate-500">
          当前筛选下没有可显示的态势对象。无坐标记录也会在此以文字形式出现。
        </p>
      )}
      {!isLoading && !error && items.length > 0 && (
        <div className="mt-4 grid gap-3 lg:grid-cols-2">
          {items.map((item) => (
            <article
              key={item.id}
              className="rounded-md border border-slate-200 bg-white p-3"
            >
              <div className="flex flex-wrap items-center gap-2">
                <Chip size="sm" variant="soft">
                  <Chip.Label>{mapObjectLabels[item.object_type]}</Chip.Label>
                </Chip>
                <Chip size="sm" variant="soft">
                  <Chip.Label>
                    {precisionLabels[item.location_precision]}
                  </Chip.Label>
                </Chip>
                <Chip size="sm" variant="soft">
                  <Chip.Label>
                    {statusLabels[item.review_status] ?? item.review_status}
                  </Chip.Label>
                </Chip>
              </div>
              <h4 className="mb-0 mt-3 text-sm font-semibold text-slate-950">
                {item.display_name ?? mapObjectLabels[item.object_type]}
              </h4>
              <p className="mb-0 mt-1 text-sm leading-6 text-slate-700">
                {item.location_text ?? "未提供文字地点；请联系指挥确认。"}
              </p>
              <dl className="mb-0 mt-3 grid grid-cols-2 gap-x-3 gap-y-1 text-xs text-slate-500">
                <div>
                  <dt className="inline">来源：</dt>
                  <dd className="inline">{item.source}</dd>
                </div>
                <div>
                  <dt className="inline">事件/上报：</dt>
                  <dd className="inline">
                    {formatDate(item.occurred_at ?? item.reported_at) ??
                      "待补充"}
                  </dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function CommandIntakePanel({
  token,
  onAccepted,
}: {
  token: string | null;
  onAccepted: () => Promise<void>;
}) {
  const [items, setItems] = useState<
    import("../api/cases").CommandIntakeCase[]
  >([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState("");
  const load = useCallback(async () => {
    if (!token) return;
    try {
      setError("");
      setItems(await listCommandIntake(token));
    } catch (cause) {
      setError(messageFrom(cause));
    }
  }, [token]);
  useEffect(() => {
    void load();
  }, [load]);
  return (
    <section
      className="mb-5 border border-slate-200 bg-white p-4"
      aria-label="待受理案件"
    >
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="m-0 text-base font-bold text-slate-950">待受理案件</h2>
          <p className="mb-0 mt-1 text-xs text-slate-600">
            仅显示接案所需的最小信息；受理后才会获得完整指挥案情权限。
          </p>
        </div>
        <Button size="sm" variant="ghost" onPress={() => void load()}>
          刷新
        </Button>
      </div>
      {error && (
        <div className="mt-3">
          <Message tone="error">{error}</Message>
        </div>
      )}
      {items.length === 0 ? (
        <p className="mb-0 mt-3 text-sm text-slate-500">当前没有待受理案件。</p>
      ) : (
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          {items.map((item) => (
            <article
              key={item.id}
              className="rounded-md border border-slate-200 p-3"
            >
              <strong className="text-sm text-slate-950">
                {item.case_code}
              </strong>
              <dl className="mb-0 mt-2 grid gap-1 text-xs text-slate-600">
                <div>
                  <dt className="inline font-medium text-slate-700">地区：</dt>
                  <dd className="inline">{item.area_hint ?? "待补充"}</dd>
                </div>
                <div>
                  <dt className="inline font-medium text-slate-700">
                    走失时间：
                  </dt>
                  <dd className="inline">{formatDate(item.last_seen_at)}</dd>
                </div>
                <div>
                  <dt className="inline font-medium text-slate-700">
                    老人年龄：
                  </dt>
                  <dd className="inline">
                    {item.elder_age == null ? "待补充" : `${item.elder_age} 岁`}
                  </dd>
                </div>
              </dl>
              <Button
                className="mt-3"
                size="sm"
                variant="primary"
                isDisabled={busy === item.id}
                onPress={() => {
                  if (!token) return;
                  setBusy(item.id);
                  acceptCommandCase(token, item.id)
                    .then(onAccepted)
                    .then(load)
                    .catch((cause) => setError(messageFrom(cause)))
                    .finally(() => setBusy(""));
                }}
              >
                受理案件
              </Button>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function CaseCollaborationPanel({
  detail,
  token,
}: {
  detail: CaseDetail;
  token: string | null;
}) {
  const [publicProgress, setPublicProgress] =
    useState<CasePublicProgress | null>(null);
  const [progressError, setProgressError] = useState("");
  const [clueDraftText, setClueDraftText] = useState("");
  const [clueDrafts, setClueDrafts] = useState<ClueDraft[]>([]);
  const [poiCategory, setPoiCategory] = useState("hospital");
  const [pois, setPois] = useState<CasePois | null>(null);
  const [summaryDraft, setSummaryDraft] = useState<SummaryDraft | null>(null);
  const [summaryVersions, setSummaryVersions] = useState<SummaryDraft[]>([]);
  const [summaryDiff, setSummaryDiff] = useState<SummaryDraftDiff | null>(null);
  const [archiveNotice, setArchiveNotice] = useState("");
  const [summary, setSummary] = useState<CaseSummary | null>(null);
  const [summaryEdit, setSummaryEdit] = useState("");
  const [reviewReason, setReviewReason] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState("");
  const progressRequestVersion = useRef(0);
  const isFamily = detail.access_role === "family";
  const canFindPois =
    detail.access_role === "commander" || detail.access_role === "volunteer";
  const isCommander = detail.access_role === "commander";

  const loadPublicProgress = useCallback(async () => {
    const requestVersion = progressRequestVersion.current + 1;
    progressRequestVersion.current = requestVersion;
    if (!token || !isFamily) {
      setPublicProgress(null);
      setProgressError("");
      return;
    }
    try {
      setProgressError("");
      const progress = await getCasePublicProgress(token, detail.id);
      if (requestVersion !== progressRequestVersion.current) return;
      setPublicProgress(progress);
    } catch (cause) {
      if (requestVersion !== progressRequestVersion.current) return;
      setProgressError(messageFrom(cause));
    }
  }, [detail.id, isFamily, token]);

  const loadSummary = useCallback(async () => {
    if (!token || !isCommander) {
      setSummary(null);
      return;
    }
    try {
      const [loadedSummary, latestDraft, versions] = await Promise.all([
        getCaseSummary(token, detail.id),
        getLatestSummaryDraft(token, detail.id),
        listSummaryDraftVersions(token, detail.id),
      ]);
      setSummary(loadedSummary);
      setSummaryDraft(latestDraft);
      setSummaryEdit(latestDraft?.content ?? "");
      setSummaryVersions(versions.items);
      setSummaryDiff(null);
    } catch (cause) {
      setError(messageFrom(cause));
    }
  }, [detail.id, isCommander, token]);

  const loadClueDrafts = useCallback(async () => {
    if (!token || !isCommander) {
      setClueDrafts([]);
      return;
    }
    try {
      setClueDrafts(await listClueDrafts(token, detail.id));
    } catch (cause) {
      setError(messageFrom(cause));
    }
  }, [detail.id, isCommander, token]);

  useEffect(() => {
    setPublicProgress(null);
    setProgressError("");
    setClueDraftText("");
    setClueDrafts([]);
    setPois(null);
    setSummaryDraft(null);
    setSummaryVersions([]);
    setSummaryDiff(null);
    setArchiveNotice("");
    setSummary(null);
    setSummaryEdit("");
    setReviewReason("");
    setError("");
    void loadPublicProgress();
    void loadSummary();
    void loadClueDrafts();
  }, [detail.id, loadClueDrafts, loadPublicProgress, loadSummary]);

  async function run(key: string, action: () => Promise<void>) {
    setBusy(key);
    setError("");
    try {
      await action();
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy("");
    }
  }

  if (!isFamily && !canFindPois && !isCommander) return null;

  return (
    <section
      className="grid gap-5 border-b border-slate-200 bg-slate-50 px-5 py-5 sm:px-6 lg:grid-cols-2"
      aria-label="协作工具"
    >
      {isFamily && (
        <div className="min-w-0">
          <div className="flex items-center justify-between gap-3">
            <h3 className="m-0 text-base font-bold text-slate-950">
              已核实公开进展
            </h3>
            <Button
              size="sm"
              variant="ghost"
              isDisabled={busy !== ""}
              onPress={() => void loadPublicProgress()}
            >
              刷新
            </Button>
          </div>
          {progressError && (
            <div className="mt-3">
              <Message tone="error">{progressError}</Message>
            </div>
          )}
          {!progressError && !publicProgress && (
            <p className="mt-3 text-sm text-slate-500">
              正在加载仅供家属查看的进展…
            </p>
          )}
          {publicProgress && (
            <div className="mt-3 space-y-3">
              <div className="rounded-md border border-emerald-200 bg-white p-3">
                <h4 className="m-0 text-sm font-semibold text-slate-900">
                  已确认信息
                </h4>
                {publicProgress.confirmed_progress.length === 0 ? (
                  <p className="mb-0 mt-2 text-sm text-slate-500">
                    暂时没有可公开的已确认信息。
                  </p>
                ) : (
                  publicProgress.confirmed_progress.map((item) => (
                    <p
                      key={item.clue_id}
                      className="mb-0 mt-2 text-sm leading-6 text-slate-700"
                    >
                      已确认一项案件进展。
                      <span className="ml-2 text-xs text-slate-500">
                        {formatDate(item.updated_at)}
                      </span>
                    </p>
                  ))
                )}
              </div>
              <div className="rounded-md border border-amber-200 bg-white p-3">
                <h4 className="m-0 text-sm font-semibold text-slate-900">
                  需要补充或核实
                </h4>
                {publicProgress.requested_family_information.length === 0 ? (
                  <p className="mb-0 mt-2 text-sm text-slate-500">
                    当前没有需要你补充的项目。
                  </p>
                ) : (
                  publicProgress.requested_family_information.map((item) => (
                    <p
                      key={item.clue_id}
                      className="mb-0 mt-2 text-sm leading-6 text-slate-700"
                    >
                      你提交的一项信息仍待补充或核实。
                      <span className="ml-2 text-xs text-slate-500">
                        {statusLabels[item.review_status] ?? item.review_status}
                      </span>
                    </p>
                  ))
                )}
              </div>
              <ul className="m-0 list-disc space-y-1 pl-5 text-xs leading-5 text-slate-600">
                {publicProgress.safety_and_contact_reminders.map((reminder) => (
                  <li key={reminder}>{reminder}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {canFindPois && (
        <div className="min-w-0">
          <h3 className="m-0 text-base font-bold text-slate-950">
            任务区域周边资源
          </h3>
          <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
            检索中心由服务端按案件任务或已确认地点确定；此处不会提交任意坐标。
          </p>
          <div className="mt-3 flex gap-2">
            <select
              aria-label="周边资源类别"
              className="min-h-10 flex-1 rounded-md border border-slate-300 bg-white px-3 text-sm"
              value={poiCategory}
              onChange={(event) => {
                setPoiCategory(event.target.value);
                setPois(null);
              }}
            >
              <option value="hospital">医院</option>
              <option value="police">派出所</option>
              <option value="transit">公交站</option>
              <option value="market">市场</option>
              <option value="community_service">社区服务中心</option>
            </select>
            <Button
              size="sm"
              variant="secondary"
              isDisabled={!token || busy === "pois"}
              onPress={() =>
                void run("pois", async () => {
                  if (!token) return;
                  setPois(await listCasePois(token, detail.id, poiCategory));
                })
              }
            >
              查询
            </Button>
          </div>
          {pois && (
            <div className="mt-3 rounded-md border border-slate-200 bg-white p-3">
              {pois.fallback_message && (
                <p className="m-0 text-xs leading-5 text-amber-800">
                  {pois.fallback_message}
                </p>
              )}
              <ul className="m-0 mt-2 space-y-2 p-0">
                {pois.items.map((item) => (
                  <li
                    key={item.id}
                    className="list-none text-sm text-slate-700"
                  >
                    <strong className="text-slate-950">{item.name}</strong>
                    {item.address && (
                      <span className="ml-2 text-xs text-slate-500">
                        {item.address}
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {isCommander && (
        <div className="min-w-0">
        <h3 className="m-0 text-base font-bold text-slate-950">
          文本整理为待审核线索
        </h3>
        <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
          只生成可回看的草稿，不会直接创建已确认线索。
        </p>
        <TextArea
          className="mt-3"
          value={clueDraftText}
          maxLength={4000}
          rows={3}
          placeholder="粘贴需要整理的受控文本"
          onChange={(event) => setClueDraftText(event.target.value)}
          fullWidth
        />
        <div className="mt-2 flex justify-end">
          <Button
            size="sm"
            variant="secondary"
            isDisabled={
              !token || !clueDraftText.trim() || busy === "clue-draft"
            }
            onPress={() =>
              void run("clue-draft", async () => {
                if (!token) return;
                const created = await createClueDraft(token, detail.id, {
                  text: clueDraftText,
                  source_type: "manual_report",
                });
                setClueDrafts((current) => [...created, ...current]);
                setClueDraftText("");
              })
            }
          >
            生成待审核草稿
          </Button>
        </div>
        {clueDrafts.map((draft) => (
          <ClueDraftReviewCard
            key={draft.id}
            draft={draft}
            isBusy={busy === `clue-draft:${draft.id}`}
            onReview={(payload) =>
              void run(`clue-draft:${draft.id}`, async () => {
                if (!token) return;
                const reviewed = await reviewClueDraft(
                  token,
                  detail.id,
                  draft.id,
                  payload,
                );
                setClueDrafts((current) =>
                  current.map((item) => (item.id === reviewed.id ? reviewed : item)),
                );
              })
            }
          />
        ))}
        </div>
      )}

      {isCommander && (
        <div className="min-w-0">
          <div className="rounded-md border border-slate-200 bg-white p-3">
            <div className="flex items-center justify-between gap-3">
              <h3 className="m-0 text-base font-bold text-slate-950">
                经授权案件摘要
              </h3>
              <Button
                size="sm"
                variant="ghost"
                isDisabled={!token || busy !== ""}
                onPress={() => void loadSummary()}
              >
                刷新摘要
              </Button>
            </div>
            {!summary ? (
              <p className="mb-0 mt-2 text-sm text-slate-500">
                正在加载服务端确定性摘要…
              </p>
            ) : (
              <>
                <p className="mb-0 mt-2 text-xs text-slate-500">
                  生成于 {formatDate(summary.generated_at)} · 来源范围：
                  {summary.source_scope.join("、")}
                </p>
                <div className="mt-3 grid gap-3 sm:grid-cols-2">
                  <div>
                    <strong className="text-sm text-slate-900">
                      最后确认信息
                    </strong>
                    <p className="mb-0 mt-1 text-sm leading-6 text-slate-700">
                      {summary.last_confirmed_information?.content ??
                        "暂无已确认信息。"}
                    </p>
                  </div>
                  <div>
                    <strong className="text-sm text-slate-900">
                      待核实事项
                    </strong>
                    <p className="mb-0 mt-1 text-sm leading-6 text-slate-700">
                      {summary.pending_verification.length === 0
                        ? "暂无。"
                        : `${summary.pending_verification.length} 项仍需人工核实，未作为确认事实发布。`}
                    </p>
                  </div>
                </div>
                {summary.safety_reminders.length > 0 && (
                  <ul className="mb-0 mt-3 list-disc space-y-1 pl-5 text-xs leading-5 text-slate-600">
                    {summary.safety_reminders.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                )}
              </>
            )}
          </div>
          <h3 className="m-0 text-base font-bold text-slate-950">
            案件摘要草稿审核
          </h3>
          <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
            线索完成人工审核后，系统会依据已授权来源范围自动生成待审核摘要；发布会替代该案件旧的已发布版本。
          </p>
          {!summaryDraft ? (
            <p className="mb-0 mt-3 text-sm text-slate-500">
              暂无待审核摘要。审核一条线索后，系统会自动生成。
            </p>
          ) : (
            <div className="mt-3 rounded-md border border-slate-200 bg-white p-3">
              <Chip size="sm" variant="soft">
                <Chip.Label>
                  {statusLabels[summaryDraft.status] ?? summaryDraft.status}
                </Chip.Label>
              </Chip>
              {summaryDraft.provider_model == null && (
                <p className="mb-0 mt-2 text-xs text-amber-800">
                  当前使用受控规则降级摘要；仍须人工审核后才能发布。
                </p>
              )}
              <p className="mb-0 mt-2 whitespace-pre-wrap text-sm leading-6 text-slate-700">
                {summaryDraft.content}
              </p>
              <Field label="编辑为新的待审核版本">
                <TextArea
                  value={summaryEdit}
                  maxLength={12000}
                  rows={5}
                  onChange={(event) => setSummaryEdit(event.target.value)}
                  fullWidth
                />
              </Field>
              <div className="mt-2 flex justify-end">
                <Button
                  size="sm"
                  variant="secondary"
                  isDisabled={!token || !summaryEdit.trim() || busy === "summary-edit"}
                  onPress={() =>
                    void run("summary-edit", async () => {
                      if (!token) return;
                      const created = await createSummaryDraft(
                        token,
                        detail.id,
                        summaryEdit,
                      );
                        setSummaryDraft(created);
                        setSummaryVersions((current) => [created, ...current]);
                    })
                  }
                >
                  创建待审核版本
                </Button>
              </div>
              {summaryDraft.status === "pending_review" && (
                <>
                  <Field label="审核理由" required>
                    <Input
                      value={reviewReason}
                      maxLength={1000}
                      onChange={(event) => setReviewReason(event.target.value)}
                      fullWidth
                    />
                  </Field>
                  <div className="mt-3 flex flex-wrap justify-end gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={
                        !token ||
                        !reviewReason.trim() ||
                        busy === "summary-review"
                      }
                      onPress={() =>
                        void run("summary-review", async () => {
                          if (!token) return;
                          setSummaryDraft(
                            await reviewSummaryDraft(
                              token,
                              detail.id,
                              summaryDraft.id,
                              { action: "reject", reason: reviewReason },
                            ),
                          );
                        })
                      }
                    >
                      驳回
                    </Button>
                    <Button
                      size="sm"
                      variant="primary"
                      isDisabled={
                        !token ||
                        !summaryDraft.publication_eligible ||
                        !reviewReason.trim() ||
                        busy === "summary-review"
                      }
                      onPress={() =>
                        void run("summary-review", async () => {
                          if (!token) return;
                          setSummaryDraft(
                            await reviewSummaryDraft(
                              token,
                              detail.id,
                              summaryDraft.id,
                              { action: "publish", reason: reviewReason },
                            ),
                          );
                        })
                      }
                    >
                      审核发布
                    </Button>
                  </div>
                </>
              )}
              {summaryDraft.status === "draft" && (
                <>
                  <Field label="提交审核理由" required>
                    <Input
                      value={reviewReason}
                      maxLength={1000}
                      onChange={(event) => setReviewReason(event.target.value)}
                      fullWidth
                    />
                  </Field>
                  <div className="mt-3 flex justify-end">
                    <Button
                      size="sm"
                      variant="primary"
                      isDisabled={!token || !reviewReason.trim() || busy === "summary-review"}
                      onPress={() =>
                        void run("summary-review", async () => {
                          if (!token) return;
                          const updated = await reviewSummaryDraft(
                            token,
                            detail.id,
                            summaryDraft.id,
                            { action: "submit", reason: reviewReason },
                          );
                          setSummaryDraft(updated);
                          setSummaryVersions((current) => current.map((item) => item.id === updated.id ? updated : item));
                        })
                      }
                    >
                      提交审核
                    </Button>
                  </div>
                </>
              )}
              {summaryDraft.status === "published" && (
                <>
                  <Field label="撤回理由" required>
                    <Input
                      value={reviewReason}
                      maxLength={1000}
                      onChange={(event) => setReviewReason(event.target.value)}
                      fullWidth
                    />
                  </Field>
                  <div className="mt-3 flex justify-end">
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={!token || !reviewReason.trim() || busy === "summary-review"}
                      onPress={() =>
                        void run("summary-review", async () => {
                          if (!token) return;
                          const updated = await reviewSummaryDraft(
                            token,
                            detail.id,
                            summaryDraft.id,
                            { action: "withdraw", reason: reviewReason },
                          );
                          setSummaryDraft(updated);
                          setSummaryVersions((current) => current.map((item) => item.id === updated.id ? updated : item));
                        })
                      }
                    >
                      撤回已发布摘要
                    </Button>
                  </div>
                </>
              )}
            </div>
          )}
          {summaryVersions.length > 1 && (
            <div className="mt-3 rounded-md border border-slate-200 bg-white p-3">
              <h4 className="m-0 text-sm font-semibold text-slate-900">版本与差异</h4>
              <div className="mt-2 flex flex-wrap gap-2">
                {summaryVersions.map((version) => (
                  <Button
                    key={version.id}
                    size="sm"
                    variant="ghost"
                    isDisabled={!token || busy === "summary-diff"}
                    onPress={() =>
                      void run("summary-diff", async () => {
                        if (!token) return;
                        const previous = summaryVersions.find((item) => item.id !== version.id);
                        if (!previous) return;
                        setSummaryDraft(version);
                        setSummaryEdit(version.content);
                        setSummaryDiff(await diffSummaryDraftVersions(token, detail.id, previous.id, version.id));
                      })
                    }
                  >
                    v{version.version} · {statusLabels[version.status] ?? version.status}
                  </Button>
                ))}
              </div>
              {summaryDiff && (
                <div className="mt-3 grid gap-3 text-xs leading-5 sm:grid-cols-2">
                  <div><strong>新增</strong><p className="mb-0 whitespace-pre-wrap">{summaryDiff.added.join("\n") || "无"}</p></div>
                  <div><strong>移除</strong><p className="mb-0 whitespace-pre-wrap">{summaryDiff.removed.join("\n") || "无"}</p></div>
                </div>
              )}
            </div>
          )}
          {(detail.status === "resolved" || detail.status === "closed") && (
            <div className="mt-3 rounded-md border border-violet-200 bg-violet-50 p-3">
              <h4 className="m-0 text-sm font-semibold text-slate-900">案例整理候选</h4>
              <p className="mb-0 mt-1 text-xs leading-5 text-slate-700">
                仅使用已确认线索与已完成任务的最小化聚合信息生成草稿；不会自动进入学习库，仍须管理员完成脱敏与版本审核。
              </p>
              <div className="mt-2 flex justify-end">
                <Button
                  size="sm"
                  variant="secondary"
                  isDisabled={!token || busy === "archive-draft"}
                  onPress={() =>
                    void run("archive-draft", async () => {
                      if (!token) return;
                      const archive = await createArchiveDraft(token, detail.id);
                      setArchiveNotice(`已创建案例整理草稿 v${archive.version}，等待管理员脱敏审核。`);
                    })
                  }
                >
                  创建案例整理草稿
                </Button>
              </div>
              {archiveNotice && <p className="mb-0 mt-2 text-xs text-violet-800">{archiveNotice}</p>}
            </div>
          )}
        </div>
      )}

      {error && (
        <div className="lg:col-span-2">
          <Message tone="error">{error}</Message>
        </div>
      )}
    </section>
  );
}

function ClueDraftReviewCard({
  draft,
  isBusy,
  onReview,
}: {
  draft: ClueDraft;
  isBusy: boolean;
  onReview: (payload: {
    action: "accept" | "reject";
    reason: string;
    candidate: ClueDraft["candidate"];
    field_decisions: Record<
      string,
      { action: "accept" | "edit" | "clear"; value?: string | null; reason?: string | null }
    >;
  }) => void;
}) {
  const [reason, setReason] = useState("");
  const [candidate, setCandidate] = useState(draft.candidate);
  useEffect(() => setCandidate(draft.candidate), [draft]);
  const fields = [
    ["时间", candidate.occurred_at],
    ["地点", candidate.location_text],
    ["来源", candidate.source_text],
    ["动作候选", candidate.action_candidates.join("；") || null],
  ];

  return (
    <div className="mt-3 rounded-md border border-amber-200 bg-white p-3">
      <p className="m-0 whitespace-pre-wrap text-sm leading-6 text-slate-700">
        {draft.content}
      </p>
      <p className="mb-0 mt-2 text-xs text-amber-800">
        {draft.uncertainty_notice}
      </p>
      <dl className="mb-0 mt-3 grid gap-2 text-sm text-slate-700">
        {fields.map(([label, value]) => (
          <div key={label}>
            <dt className="inline font-medium text-slate-900">{label}：</dt>
            <dd className="inline">
              {value || "需要人工补充"}
            </dd>
          </div>
        ))}
      </dl>
      <p className="mb-0 mt-2 text-xs text-slate-500">
        来源片段：{candidate.source_excerpt}
      </p>
      {Object.entries(candidate.field_sources).length > 0 && (
        <ul className="mb-0 mt-2 space-y-1 p-0 text-xs text-slate-500">
          {Object.entries(candidate.field_sources).map(([field, source]) => (
            <li key={field} className="list-none">
              {field}：{source.excerpt ?? "未提供来源片段"}
              {source.reference ? `（${source.reference}）` : ""}
            </li>
          ))}
        </ul>
      )}
      {draft.review_status === "pending_review" ? (
        <>
          <Field label="时间候选">
            <Input
              value={candidate.occurred_at ?? ""}
              maxLength={80}
              onChange={(event) =>
                setCandidate((current) => ({
                  ...current,
                  occurred_at: event.target.value || null,
                }))
              }
              fullWidth
            />
          </Field>
          <Field label="地点候选">
            <Input
              value={candidate.location_text ?? ""}
              maxLength={500}
              onChange={(event) =>
                setCandidate((current) => ({
                  ...current,
                  location_text: event.target.value || null,
                }))
              }
              fullWidth
            />
          </Field>
          <Field label="来源候选">
            <Input
              value={candidate.source_text ?? ""}
              maxLength={300}
              onChange={(event) =>
                setCandidate((current) => ({
                  ...current,
                  source_text: event.target.value || null,
                }))
              }
              fullWidth
            />
          </Field>
          <Field label="动作候选（每行一项）">
            <TextArea
              value={candidate.action_candidates.join("\n")}
              maxLength={2400}
              rows={3}
              onChange={(event) =>
                setCandidate((current) => ({
                  ...current,
                  action_candidates: event.target.value
                    .split("\n")
                    .map((value) => value.trim())
                    .filter(Boolean),
                }))
              }
              fullWidth
            />
          </Field>
          <Field label="草稿审核理由" required>
            <Input
              value={reason}
              maxLength={1000}
              onChange={(event) => setReason(event.target.value)}
              fullWidth
            />
          </Field>
          <div className="mt-3 flex justify-end gap-2">
            <Button
              size="sm"
              variant="ghost"
              isDisabled={!reason.trim() || isBusy}
              onPress={() => onReview({
                action: "reject",
                reason,
                candidate,
                field_decisions: fieldDecisions(draft.candidate, candidate, reason),
              })}
            >
              拒绝候选
            </Button>
            <Button
              size="sm"
              variant="secondary"
              isDisabled={!reason.trim() || isBusy}
              onPress={() => onReview({
                action: "accept",
                reason,
                candidate,
                field_decisions: fieldDecisions(draft.candidate, candidate, reason),
              })}
            >
              人工接受候选
            </Button>
          </div>
        </>
      ) : (
        <p className="mb-0 mt-3 text-xs text-slate-600">
          已{draft.review_status === "accepted" ? "接受" : "拒绝"}；正式线索仍需按既有流程人工审核。
        </p>
      )}
    </div>
  );
}

function fieldDecisions(
  original: ClueDraft["candidate"],
  edited: ClueDraft["candidate"],
  reason: string,
) {
  const scalarFields = ["content_summary", "occurred_at", "location_text", "source_text"] as const;
  const decisions: Record<string, { action: "accept" | "edit" | "clear"; value?: string | null; reason?: string }> = {};
  for (const field of scalarFields) {
    const before = original[field];
    const after = edited[field];
    decisions[field] = before === after
      ? { action: "accept", reason }
      : after == null || after === ""
        ? { action: "clear", reason }
        : { action: "edit", value: after, reason };
  }
  const beforeActions = original.action_candidates.join("\n");
  const afterActions = edited.action_candidates.join("\n");
  decisions.action_candidates = beforeActions === afterActions
    ? { action: "accept", reason }
    : afterActions
      ? { action: "edit", value: afterActions, reason }
      : { action: "clear", reason };
  return decisions;
}

function ReviewButton({
  label,
  status,
  clueId,
  reason,
  relatedClueId,
  busy,
  run,
  onReviewed,
}: {
  label: string;
  status: ClueReviewStatus;
  clueId: string;
  reason: string;
  relatedClueId: string;
  busy: string;
  run: (
    key: string,
    action: () => Promise<unknown>,
    message: string,
  ) => Promise<boolean>;
  onReviewed: () => void;
}) {
  const { token } = useAuth();
  const key = `review:${clueId}:${status}`;
  const requiresRelationship =
    status === "duplicate" || status === "conflicting";
  const isReady =
    Boolean(reason.trim()) &&
    (!requiresRelationship || Boolean(relatedClueId.trim()));
  const [isConfirming, setIsConfirming] = useState(false);
  const reviewTriggerRef = useRef<HTMLButtonElement>(null);
  const hadConfirmationOpen = useRef(false);

  useEffect(() => {
    if (isConfirming) {
      hadConfirmationOpen.current = true;
      return;
    }
    if (hadConfirmationOpen.current) {
      reviewTriggerRef.current?.focus();
      hadConfirmationOpen.current = false;
    }
  }, [isConfirming]);

  function submitReview() {
    const normalizedReason = reason.trim();
    if (
      !token ||
      !normalizedReason ||
      (requiresRelationship && !relatedClueId.trim())
    )
      return;
    void run(
      key,
      () =>
        reviewClue(token, clueId, {
          status,
          reason: normalizedReason,
          related_clue_id: requiresRelationship ? relatedClueId.trim() : null,
          relationship_type:
            status === "duplicate"
              ? "duplicate_of"
              : status === "conflicting"
                ? "conflicts_with"
                : null,
        }),
      `线索已更新为${statusLabels[status]}`,
    ).then((succeeded) => {
      if (succeeded) onReviewed();
      setIsConfirming(false);
    });
  }
  return (
    <AlertDialog isOpen={isConfirming} onOpenChange={setIsConfirming}>
      <Button
        ref={reviewTriggerRef}
        size="sm"
        variant={status === "confirmed" ? "secondary" : "ghost"}
        isDisabled={busy === key || !isReady}
        onPress={() => setIsConfirming(true)}
      >
        {status === "confirmed" && <CheckCircle2 size={15} />}
        {label}
      </Button>
      <AlertDialog.Backdrop
        className="fixed inset-0 z-50 bg-slate-950/40 p-4"
        isKeyboardDismissDisabled={false}
      >
        <AlertDialog.Container className="grid min-h-full place-items-center">
          <AlertDialog.Dialog
            role="dialog"
            aria-labelledby={`review-confirmation-${clueId}`}
            className="w-full max-w-md rounded-md bg-white p-5 shadow-xl"
          >
            <h4
              id={`review-confirmation-${clueId}`}
              className="m-0 text-base font-bold text-slate-950"
            >
              确认审核操作
            </h4>
            <p className="mb-0 mt-3 text-sm leading-6 text-slate-700">
              将把该线索从“{statusLabels.pending_review}”改为“
              {statusLabels[status]}
              ”。此操作会记录审核理由和操作者，并更新其他角色可见的内容。
            </p>
            <div className="mt-5 flex justify-end gap-2">
              <Button
                size="sm"
                variant="ghost"
                isDisabled={busy === key}
                onPress={() => setIsConfirming(false)}
              >
                取消
              </Button>
              <Button
                size="sm"
                variant="primary"
                isDisabled={busy === key}
                onPress={submitReview}
              >
                确认提交
              </Button>
            </div>
          </AlertDialog.Dialog>
        </AlertDialog.Container>
      </AlertDialog.Backdrop>
    </AlertDialog>
  );
}

function RelatedCluePicker({
  clueId,
  candidates,
  selectedId,
  query,
  onChange,
}: {
  clueId: string;
  candidates: Clue[];
  selectedId: string;
  query: string;
  onChange: (relatedClueId: string, relatedClueQuery: string) => void;
}) {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const options = candidates.filter((candidate) => {
    if (candidate.id === clueId) return false;
    if (candidate.id === selectedId || !normalizedQuery) return true;
    return [
      candidate.content,
      candidate.status,
      candidate.location_text,
      candidate.reported_at,
    ]
      .filter((value): value is string => Boolean(value))
      .join(" ")
      .toLocaleLowerCase()
      .includes(normalizedQuery);
  });

  return (
    <div className="grid gap-2">
      <Input
        aria-label="搜索关联线索"
        value={query}
        maxLength={200}
        placeholder="按线索内容、状态、地点或时间筛选"
        onChange={(event) => onChange(selectedId, event.target.value)}
        fullWidth
      />
      <select
        aria-label="选择关联线索"
        className="min-h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm"
        value={selectedId}
        onChange={(event) => onChange(event.target.value, query)}
      >
        <option value="">请选择关联线索</option>
        {options.map((candidate) => (
          <option key={candidate.id} value={candidate.id}>
            {clueOptionLabel(candidate)}
          </option>
        ))}
      </select>
      <p className="m-0 text-xs leading-5 text-slate-600">
        {options.length === 0
          ? "没有匹配的案件线索。"
          : `显示 ${options.length} 条可选线索；不会提交或显示 UUID。`}
      </p>
    </div>
  );
}

function clueOptionLabel(clue: Clue) {
  const excerpt = clue.content.replace(/\s+/g, " ").slice(0, 80);
  const location = clue.location_text ? ` · ${clue.location_text}` : "";
  return `${statusLabels[clue.status] ?? clue.status} · ${formatDate(clue.reported_at) ?? "时间未知"} · ${excerpt}${location}`;
}

function ElderProfileEditor({
  detail,
  token,
  busy,
  onSave,
}: {
  detail: CaseDetail;
  token: string | null;
  busy: string;
  onSave: (
    payload: import("../api/cases").UpdateElderProfilePayload,
  ) => Promise<boolean>;
}) {
  const [ageError, setAgeError] = useState("");
  const [draft, setDraft] = useState({
    display_name: detail.elder_profile.display_name,
    age:
      detail.elder_profile.age == null ? "" : String(detail.elder_profile.age),
    gender: detail.elder_profile.gender ?? "",
    physical_description: detail.elder_profile.physical_description ?? "",
    clothing_description: detail.elder_profile.clothing_description ?? "",
    health_notes: detail.elder_profile.health_notes ?? "",
    last_seen_at: detail.elder_profile.last_seen_at ?? "",
    last_seen_location: detail.elder_profile.last_seen_location ?? "",
  });
  useEffect(
    () =>
      setDraft({
        display_name: detail.elder_profile.display_name,
        age:
          detail.elder_profile.age == null
            ? ""
            : String(detail.elder_profile.age),
        gender: detail.elder_profile.gender ?? "",
        physical_description: detail.elder_profile.physical_description ?? "",
        clothing_description: detail.elder_profile.clothing_description ?? "",
        health_notes: detail.elder_profile.health_notes ?? "",
        last_seen_at: detail.elder_profile.last_seen_at ?? "",
        last_seen_location: detail.elder_profile.last_seen_location ?? "",
      }),
    [detail],
  );
  const field = (key: keyof typeof draft, label: string, multiline = false) => (
    <Field label={label}>
      {multiline ? (
        <TextArea
          value={draft[key]}
          onChange={(event) =>
            setDraft((value) => ({ ...value, [key]: event.target.value }))
          }
          fullWidth
        />
      ) : (
        <Input
          type={key === "age" ? "number" : undefined}
          min={key === "age" ? 0 : undefined}
          max={key === "age" ? 130 : undefined}
          value={draft[key]}
          onChange={(event) => {
            setAgeError("");
            setDraft((value) => ({ ...value, [key]: event.target.value }));
          }}
          fullWidth
        />
      )}
    </Field>
  );
  return (
    <form
      className="border-b border-slate-200 bg-brand-50/30 px-5 py-5 sm:px-6"
      onSubmit={(event) => {
        event.preventDefault();
        if (!token) return;
        const ageText = draft.age.trim();
        const age = ageText === "" ? undefined : Number(ageText);
        if (
          age !== undefined &&
          (!Number.isInteger(age) || age < 0 || age > 130)
        ) {
          setAgeError("年龄必须是 0 到 130 之间的整数。");
          return;
        }
        void onSave({
          display_name: draft.display_name,
          age,
          gender: draft.gender,
          physical_description: draft.physical_description,
          clothing_description: draft.clothing_description,
          health_notes: draft.health_notes,
          last_seen_at: draft.last_seen_at,
          last_seen_location: draft.last_seen_location,
        });
      }}
    >
      {ageError && (
        <p
          className="mb-3 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700"
          role="alert"
        >
          {ageError}
        </p>
      )}
      <h3 className="m-0 text-sm font-bold text-slate-950">
        人物摘要补充 / 更正
      </h3>
      <p className="mt-1 text-xs text-slate-600">
        每次保存都会保留前后版本；案件状态、成员和任务信息不在此表单中。
      </p>
      <div className="mt-3 grid gap-3 sm:grid-cols-2">
        {field("display_name", "姓名")} {field("age", "年龄")}{" "}
        {field("gender", "性别")} {field("last_seen_location", "最后出现地点")}{" "}
        {field("last_seen_at", "最后出现时间")}{" "}
        {field("physical_description", "体貌", true)}{" "}
        {field("clothing_description", "衣着", true)}{" "}
        {field("health_notes", "健康注意", true)}
      </div>
      <div className="mt-4 flex justify-end">
        <Button
          type="submit"
          size="sm"
          variant="secondary"
          isDisabled={busy === "elder-profile"}
        >
          {busy === "elder-profile" ? "正在保存" : "保存人物摘要"}
        </Button>
      </div>
    </form>
  );
}

function Field({
  label,
  required,
  children,
}: {
  label: string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-xs font-semibold text-slate-600">
        {label}
        {required ? " *" : ""}
      </span>
      {children}
    </label>
  );
}

function Info({
  label,
  value,
  icon,
}: {
  label: string;
  value: string | null;
  icon?: React.ReactNode;
}) {
  return (
    <div className="min-w-0">
      <span className="flex items-center gap-1 text-xs text-slate-500">
        {icon}
        {label}
      </span>
      <strong className="mt-1 block whitespace-pre-wrap text-sm font-medium text-slate-800">
        {value || "未填写"}
      </strong>
    </div>
  );
}

function Message({
  tone,
  children,
}: {
  tone: "error" | "success";
  children: React.ReactNode;
}) {
  return (
    <div
      className={`mb-3 rounded-md border px-3 py-2 text-sm ${tone === "error" ? "border-red-200 bg-red-50 text-red-700" : "border-emerald-200 bg-emerald-50 text-emerald-700"}`}
      role={tone === "error" ? "alert" : "status"}
    >
      {children}
    </div>
  );
}

function nullable(value: string): string | null {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function toIsoOrNull(value: string): string | null {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date.toISOString();
}

function messageFrom(cause: unknown): string {
  if (cause instanceof ApiClientError) return cause.message;
  return cause instanceof Error ? cause.message : "操作失败";
}

function formatDate(value: string | null): string | null {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : date.toLocaleString("zh-CN", { hour12: false });
}

function toDateTimeLocal(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

function formatBytes(value: number): string {
  return value >= 1024 * 1024
    ? `${(value / (1024 * 1024)).toFixed(1)} MiB`
    : `${value} 字节`;
}
