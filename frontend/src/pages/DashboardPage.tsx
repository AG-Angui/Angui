import { Button, Card, Chip, Input, TextArea } from "@heroui/react";
import {
  ClipboardCheck,
  FileSearch,
  RadioTower,
  RefreshCw,
  UsersRound,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  deidentifyArchiveDraft,
  getCase,
  listAdminArchiveDrafts,
  listCases,
  listCommandIntake,
  reviewArchiveDraft,
} from "../api/cases";
import type {
  ArchiveDraft,
  CaseDetail,
  CaseListItem,
  CommandIntakeCase,
} from "../api/cases";
import { useAuth } from "../auth/useAuth";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../components/ContentState";
import { ServiceStatus } from "../components/ServiceStatus";

const caseStatusLabels: Record<string, string> = {
  active: "进行中",
  resolved: "已找到",
  closed: "已关闭",
};

export function DashboardPage() {
  const { token, user } = useAuth();
  const isCommander = user?.global_capabilities.includes("commander") ?? false;
  const isAdmin = user?.global_capabilities.includes("admin") ?? false;
  const [cases, setCases] = useState<CaseListItem[]>([]);
  const [details, setDetails] = useState<CaseDetail[]>([]);
  const [pendingIntakeCases, setPendingIntakeCases] = useState<
    CommandIntakeCase[]
  >([]);
  const [archiveDrafts, setArchiveDrafts] = useState<ArchiveDraft[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState("");

  const loadDashboard = useCallback(async () => {
    if (!token) return;
    setIsLoading(true);
    setError("");
    try {
      const [items, intakeCases, archives] = await Promise.all([
        listCases(token),
        isCommander ? listCommandIntake(token) : Promise.resolve([]),
        isAdmin ? listAdminArchiveDrafts(token) : Promise.resolve([]),
      ]);
      const settledDetails = await Promise.allSettled(
        items.slice(0, 20).map((item) => getCase(token, item.id)),
      );
      const loaded = settledDetails.flatMap((result) =>
        result.status === "fulfilled" ? [result.value] : [],
      );

      setCases(items);
      setDetails(loaded);
      setPendingIntakeCases(intakeCases);
      setArchiveDrafts(archives);
      if (loaded.length !== items.length) {
        setError("部分案件详情暂时不可用，统计数据可能不完整。");
      }
    } catch (cause) {
      setCases([]);
      setDetails([]);
      setPendingIntakeCases([]);
      setArchiveDrafts([]);
      setError(cause instanceof Error ? cause.message : "无法连接案件服务。");
    } finally {
      setIsLoading(false);
    }
  }, [isAdmin, isCommander, token]);

  useEffect(() => {
    void loadDashboard();
  }, [loadDashboard]);

  const pendingClues = useMemo(
    () =>
      details
        .flatMap((item) => item.clues)
        .filter((clue) => clue.status === "pending_review").length,
    [details],
  );
  const activeCases = cases.filter((item) => item.status === "active").length;
  const confirmedClues = details
    .flatMap((item) => item.clues)
    .filter((clue) => clue.status === "confirmed").length;
  const statusRecordCount = cases.length + pendingIntakeCases.length;
  const emptyState =
    user?.account_type === "learner"
      ? {
          title: "新人账号暂未获得案件权限",
          description:
            "当前后端只会返回你作为案件成员可访问的案件；学习模块尚未提供接口。",
        }
      : user?.global_capabilities.includes("admin")
        ? {
            title: "管理员账号不自动拥有案件权限",
            description:
              "管理员需要先被授予具体案件成员关系，才能查看案件内容。",
          }
        : {
            title: "当前没有可访问案件",
            description: "创建案件或由案件成员邀请后，案件会显示在这里。",
          };

  const metrics = [
    {
      label: "活动案件",
      value: activeCases,
      icon: RadioTower,
      iconClass: "bg-red-50 text-red-700",
    },
    ...(isCommander
      ? [
          {
            label: "待受理案件",
            value: pendingIntakeCases.length,
            icon: RadioTower,
            iconClass: "bg-violet-50 text-violet-700",
          },
        ]
      : []),
    {
      label: "待审核线索",
      value: pendingClues,
      icon: FileSearch,
      iconClass: "bg-amber-50 text-amber-700",
    },
    {
      label: "已确认线索",
      value: confirmedClues,
      icon: ClipboardCheck,
      iconClass: "bg-blue-50 text-blue-700",
    },
    {
      label: "可访问案件",
      value: cases.length,
      icon: UsersRound,
      iconClass: "bg-emerald-50 text-emerald-700",
    },
  ];

  return (
    <div className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 flex min-h-14 flex-col items-start justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <span className="mb-1 block text-xs font-semibold text-slate-500">
            {user?.display_name}
          </span>
          <h1 className="m-0 text-2xl font-bold text-slate-950 lg:text-3xl">
            行动总览
          </h1>
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="ghost"
            isDisabled={isLoading}
            onPress={() => void loadDashboard()}
          >
            <RefreshCw size={16} aria-hidden="true" />
            刷新
          </Button>
          <ServiceStatus />
        </div>
      </header>

      {error && !isLoading && cases.length > 0 && (
        <div
          className="mb-5 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800"
          role="status"
        >
          {error}
        </div>
      )}

      {isLoading ? (
        <section
          className="border-y border-slate-200 bg-white"
          aria-label="正在加载行动总览"
        >
          <LoadingState label="正在加载案件和线索统计" />
        </section>
      ) : error && cases.length === 0 ? (
        <section
          className="border-y border-slate-200 bg-white"
          aria-label="行动总览加载失败"
        >
          <ErrorState message={error} onRetry={() => void loadDashboard()} />
        </section>
      ) : (
        <>
          <section
            className="mb-7 grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-5 lg:gap-3"
            aria-label="行动指标"
          >
            {metrics.map(({ label, value, icon: Icon, iconClass }) => (
              <Card
                key={label}
                className="min-h-24 rounded-md! border border-slate-200 shadow-none"
              >
                <Card.Content className="flex h-full flex-row! items-center! gap-3 p-4">
                  <span
                    className={`grid size-10 shrink-0 place-items-center rounded-md ${iconClass}`}
                    aria-hidden="true"
                  >
                    <Icon size={19} />
                  </span>
                  <div className="min-w-0">
                    <span className="block whitespace-nowrap text-xs text-slate-500">
                      {label}
                    </span>
                    <strong className="mt-1 block text-2xl leading-none text-slate-950">
                      {value}
                    </strong>
                  </div>
                </Card.Content>
              </Card>
            ))}
          </section>

          {isAdmin && (
            <section
              className="mb-7 overflow-hidden border-y border-violet-200 bg-violet-50"
              aria-labelledby="archive-review-title"
            >
              <header className="border-b border-violet-200 px-5 py-4">
                <span className="block text-xs font-semibold text-violet-700">
                  管理员审核
                </span>
                <h2
                  id="archive-review-title"
                  className="m-0 mt-1 text-base font-bold text-slate-950"
                >
                  案例整理草稿
                </h2>
                <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
                  必须先完成脱敏确认，才可发布为学习资源；撤回不影响原案件。
                </p>
              </header>
              {archiveDrafts.length === 0 ? (
                <p className="m-0 px-5 py-4 text-sm text-slate-600">
                  暂无需要审核的案例整理草稿。
                </p>
              ) : (
                <div className="grid gap-3 p-5 lg:grid-cols-2">
                  {archiveDrafts.map((draft) => (
                    <ArchiveReviewCard
                      key={draft.id}
                      draft={draft}
                      token={token}
                      onChanged={(updated) =>
                        setArchiveDrafts((current) =>
                          current.map((item) =>
                            item.id === updated.id ? updated : item,
                          ),
                        )
                      }
                    />
                  ))}
                </div>
              )}
            </section>
          )}

          <section
            className="overflow-hidden border-y border-slate-200 bg-white"
            aria-labelledby="case-status-title"
          >
            <header className="flex min-h-18 items-center justify-between gap-5 border-b border-slate-200 px-5 py-4">
              <div>
                <span className="mb-0.5 block text-xs font-semibold text-slate-500">
                  实时状态
                </span>
                <h2
                  id="case-status-title"
                  className="m-0 text-base font-bold text-slate-950"
                >
                  案件态势
                </h2>
              </div>
              <Chip size="sm" variant="soft">
                <Chip.Label>{statusRecordCount} 条记录</Chip.Label>
              </Chip>
            </header>
            {statusRecordCount === 0 ? (
              <EmptyState icon={RadioTower} {...emptyState} />
            ) : (
              <div className="divide-y divide-slate-100">
                {pendingIntakeCases.map((item) => (
                  <div
                    key={item.id}
                    className="grid gap-2 bg-violet-50/50 px-5 py-3 sm:grid-cols-[140px_minmax(0,1fr)_auto] sm:items-center"
                  >
                    <strong className="text-sm text-slate-950">
                      {item.case_code}
                    </strong>
                    <span className="text-sm text-slate-600">
                      地区：{item.area_hint ?? "待补充"} · 走失时间：
                      {item.last_seen_at ?? "待补充"} · 老人年龄：
                      {item.elder_age == null
                        ? "待补充"
                        : `${item.elder_age} 岁`}
                    </span>
                    <Chip size="sm" variant="soft">
                      <Chip.Label>待受理</Chip.Label>
                    </Chip>
                  </div>
                ))}
                {cases.slice(0, 6).map((item) => (
                  <div
                    key={item.id}
                    className="grid gap-2 px-5 py-3 sm:grid-cols-[140px_minmax(0,1fr)_auto] sm:items-center"
                  >
                    <strong className="text-sm text-slate-950">
                      {item.case_code}
                    </strong>
                    <span className="truncate text-sm text-slate-600">
                      {item.display_name} ·{" "}
                      {item.last_seen_location ?? "地点待补充"}
                    </span>
                    <Chip size="sm" variant="soft">
                      <Chip.Label>
                        {caseStatusLabels[item.status] ?? item.status}
                      </Chip.Label>
                    </Chip>
                  </div>
                ))}
              </div>
            )}
          </section>
        </>
      )}
    </div>
  );
}

function ArchiveReviewCard({
  draft,
  token,
  onChanged,
}: {
  draft: ArchiveDraft;
  token: string | null;
  onChanged: (draft: ArchiveDraft) => void;
}) {
  const [reason, setReason] = useState("");
  const [deidentifiedMaterial, setDeidentifiedMaterial] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const action = async (run: () => Promise<ArchiveDraft>) => {
    if (!token || !reason.trim()) return;
    setBusy(true);
    setError("");
    try {
      onChanged(await run());
      setReason("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "案例草稿审核失败。");
    } finally {
      setBusy(false);
    }
  };
  return (
    <article className="rounded-md border border-violet-200 bg-white p-4">
      <div className="flex items-center justify-between gap-3">
        <strong className="text-sm text-slate-950">
          草稿 v{draft.version}
        </strong>
        <Chip size="sm" variant="soft">
          <Chip.Label>{draft.status}</Chip.Label>
        </Chip>
      </div>
      <p className="mb-0 mt-2 whitespace-pre-wrap text-sm leading-6 text-slate-700">
        {draft.content}
      </p>
      <p className="mb-0 mt-2 text-xs text-slate-500">
        来源范围：{draft.source_scope.join("、")} · 脱敏：
        {draft.deidentification_status}
      </p>
      {draft.status !== "rejected" && draft.status !== "withdrawn" && (
        <>
          <Input
            className="mt-3"
            aria-label={`案例草稿 ${draft.id} 审核理由`}
            value={reason}
            maxLength={1000}
            onChange={(event) => setReason(event.target.value)}
          />
          <div className="mt-3 flex flex-wrap justify-end gap-2">
            {draft.status === "draft" && (
              <>
                <TextArea
                  className="mt-3"
                  aria-label={`脱敏材料 ${draft.id}`}
                  value={deidentifiedMaterial}
                  maxLength={12000}
                  rows={5}
                  onChange={(event) =>
                    setDeidentifiedMaterial(event.target.value)
                  }
                />
                <Button
                  size="sm"
                  variant="ghost"
                  isDisabled={busy || !reason.trim()}
                  onPress={() =>
                    void action(() =>
                      deidentifyArchiveDraft(token!, draft.id, {
                        outcome: "reject",
                        reason,
                      }),
                    )
                  }
                >
                  拒绝脱敏
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  isDisabled={
                    busy || !reason.trim() || !deidentifiedMaterial.trim()
                  }
                  onPress={() =>
                    void action(() =>
                      deidentifyArchiveDraft(token!, draft.id, {
                        outcome: "confirm",
                        reason,
                        deidentified_material: deidentifiedMaterial,
                      }),
                    )
                  }
                >
                  确认已脱敏
                </Button>
              </>
            )}
            {draft.status === "pending_review" && (
              <>
                <Button
                  size="sm"
                  variant="ghost"
                  isDisabled={busy || !reason.trim()}
                  onPress={() =>
                    void action(() =>
                      reviewArchiveDraft(token!, draft.id, {
                        action: "reject",
                        reason,
                      }),
                    )
                  }
                >
                  驳回
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  isDisabled={busy || !reason.trim()}
                  onPress={() =>
                    void action(() =>
                      reviewArchiveDraft(token!, draft.id, {
                        action: "publish",
                        reason,
                      }),
                    )
                  }
                >
                  审核发布
                </Button>
              </>
            )}
            {draft.status === "published" && (
              <Button
                size="sm"
                variant="ghost"
                isDisabled={busy || !reason.trim()}
                onPress={() =>
                  void action(() =>
                    reviewArchiveDraft(token!, draft.id, {
                      action: "withdraw",
                      reason,
                    }),
                  )
                }
              >
                撤回
              </Button>
            )}
          </div>
        </>
      )}
      {error && <p className="mb-0 mt-2 text-xs text-red-700">{error}</p>}
    </article>
  );
}
