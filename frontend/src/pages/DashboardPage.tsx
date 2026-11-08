import { Button, Card, Chip, Input, TextArea } from "@heroui/react";
import { ArrowRight, BookOpen, ClipboardCheck, Clock3, FileSearch, HeartHandshake, ListChecks, Navigation, RadioTower, RefreshCw, ShieldCheck, UserRound, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router";
import { workspaceAccess, workspaceIdentity } from "../workspaceNavigation";
import { useDashboardData } from "./useDashboardData";
import "./DashboardPage.css";
import { useCallback, useEffect, useState } from "react";
import {
  deidentifyArchiveDraft,
  diffArchiveReviewMaterials,
  listArchiveReviewMaterials,
  reviewArchiveDraft,
  restoreArchiveReviewMaterial,
} from "../api/cases";
import type {
  ArchiveReviewMaterial,
  ArchiveDraft,
  CaseListItem,
} from "../api/cases";
import { useAuth } from "../auth/useAuth";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../components/ContentState";

const caseStatusLabels: Record<string, string> = {
  active: "进行中",
  ended: "已结束",
  reviewing: "复盘中",
  archived: "已归档",
  resolved: "已找到",
  closed: "已关闭",
};

type Entry = { title: string; description: string; to: string; icon: LucideIcon };
type Todo = { title: string; description: string; to: string; icon: LucideIcon; count: number | null; attention?: boolean };

function DashboardPanel({ title, id, description, children }: { title: string; id: string; description?: string; children: ReactNode }) {
  return <Card className="dashboard-panel" role="region" aria-labelledby={id}>
    <Card.Content className="dashboard-panel-content">
      <header className="dashboard-panel-heading"><h2 id={id}>{title}</h2>{description && <p>{description}</p>}</header>
      {children}
    </Card.Content>
  </Card>;
}

function displayTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "时间待补充" : new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false }).format(date);
}

export function DashboardPage() {
  const { token, user } = useAuth();
  const access = workspaceAccess(user);
  const data = useDashboardData();
  const entries: Entry[] = [
    ...(access.family ? [
      { title: "发起求助", description: "登记走失信息，继续未完成的求助。", to: "/family/intake", icon: HeartHandshake },
      { title: "公开进展", description: "查看关联案件的已确认进展与联系提醒。", to: "/family", icon: RadioTower },
    ] : []),
    ...(access.commander ? [{ title: "案件与线索", description: "受理案件、审核线索，查看任务安全提示。", to: "/command", icon: RadioTower }] : []),
    ...(access.volunteer ? [{ title: "我的任务", description: "查看分派与协作任务，按安全提示参与行动。", to: "/volunteer", icon: Navigation }] : []),
    ...(access.admin ? [
      { title: "账号审核", description: "审核已验证邮箱的访问申请。", to: "/admin/access-requests", icon: ShieldCheck },
      { title: "内容治理", description: "维护学习资料、题库与知识内容。", to: "/admin/learning", icon: BookOpen },
      { title: "归档审核", description: "核对脱敏材料、审核与发布案例草稿。", to: "#archive-review", icon: FileSearch },
    ] : []),
    ...(access.learning ? [{ title: "学习中心", description: "阅读业务资料、练习题目，回顾个人学习。", to: "/learning", icon: BookOpen }] : []),
    { title: "个人资料", description: "维护个人信息与工作偏好。", to: "/profile", icon: UserRound },
  ];
  const activeTasks = data.tasks.items.filter((task) => ["assigned", "accepted", "active", "blocked"].includes(task.status));
  const reviewCount = data.reviews.items.flatMap((item) => item.clues).filter((clue) => clue.status === "pending_review").length;
  const firstReviewCase = data.reviews.items.find((item) => item.clues.some((clue) => clue.status === "pending_review"));
  const todos: Todo[] = [
    ...(access.family ? [{ title: "继续求助登记", description: data.sessions.error || "本人尚未完成的登记与信息确认。", to: data.sessions.items[0] ? `/family/intake?session_id=${encodeURIComponent(data.sessions.items[0].id)}` : "/family/intake", icon: HeartHandshake, count: data.sessions.error ? null : data.sessions.items.length }] : []),
    ...(access.commander ? [
      { title: "待受理案件", description: data.intake.error || "尚未有指挥人员受理的案件，进入指挥端处理。", to: "/command", icon: RadioTower, count: data.intake.error ? null : data.intake.items.length, attention: true },
      { title: "待审核线索", description: data.reviews.error || (data.reviews.partial ? "仅已加载的前20个指挥成员案件，非全量统计。" : "仅统计本人具有指挥成员权限的案件。"), to: firstReviewCase ? `/command/cases/${firstReviewCase.id}#case-clues` : "/command", icon: FileSearch, count: data.reviews.error ? null : reviewCount, attention: true },
    ] : []),
    ...(access.volunteer ? [{ title: "待处理任务", description: data.tasks.error || "包含本人分派及协作任务；不含已完成或已取消任务。", to: "/volunteer", icon: ListChecks, count: data.tasks.error ? null : activeTasks.length }] : []),
    ...(access.admin ? [
      { title: "待审核账号", description: data.requests.error || "已完成邮箱验证、等待管理员审核的申请。", to: "/admin/access-requests", icon: ShieldCheck, count: data.requests.error ? null : data.requests.items.filter((item) => item.status === "pending_review").length, attention: true },
      { title: "待处理归档", description: data.archives.error || "待脱敏或待审核的案例草稿，不含已发布材料。", to: "#archive-review", icon: ClipboardCheck, count: data.archives.error ? null : data.archives.items.filter((item) => item.status === "draft" || item.status === "pending_review").length },
    ] : []),
  ];
  const caseHref = (item: CaseListItem) => access.family && item.access_role === "family" ? `/family/cases/${item.id}` : access.commander && item.access_role === "commander" ? `/command/cases/${item.id}` : access.volunteer && item.access_role === "volunteer" ? "/volunteer" : undefined;
  const caseDescription = user?.account_type === "learner" ? "新人账号暂未获得案件权限" : access.admin ? "管理员账号不自动拥有案件权限" : "当前没有可访问案件";
  const sourceErrors = [data.cases.error, data.intake.error, data.sessions.error, data.tasks.error, data.archives.error, data.requests.error, data.reviews.error].filter(Boolean);
  const recent = [
    ...data.cases.items.map((item) => ({ key: `case-${item.id}`, title: item.case_code, description: `案件更新 · ${caseStatusLabels[item.status] ?? item.status}`, at: item.updated_at, to: caseHref(item) })),
    ...data.tasks.items.map((item) => ({ key: `task-${item.id}`, title: item.title, description: "任务记录更新", at: item.updated_at, to: "/volunteer" })),
    ...data.sessions.items.filter((item) => item.updated_at).map((item) => ({ key: `session-${item.id}`, title: "求助登记", description: "尚未完成的登记记录更新", at: item.updated_at!, to: `/family/intake?session_id=${encodeURIComponent(item.id)}` })),
  ].sort((a, b) => Date.parse(b.at) - Date.parse(a.at)).slice(0, 5);

  return <div className="dashboard-page">
    <header className="dashboard-header">
      <div><div className="dashboard-identity"><span>{workspaceIdentity(user)}</span><span>当前工作区</span></div>
        <h1>总览首页</h1><p>从当前身份出发，先处理重要事项，再查看工作进展。</p>
      </div>
      <Button variant="secondary" className="dashboard-refresh" isDisabled={data.loading} onPress={() => void data.load()}><RefreshCw size={16} aria-hidden="true" />刷新数据</Button>
    </header>

    <section className="dashboard-core" aria-labelledby="dashboard-core-title">
      <h2 id="dashboard-core-title" className="dashboard-section-title">核心工作入口</h2>
      <div className={`dashboard-entry-grid entry-count-${Math.min(entries.length, 4)}`}>
        {entries.map(({ title, description, to, icon: Icon }, index) => {
          const content = <><span className="dashboard-entry-icon" aria-hidden="true"><Icon size={22} /></span><div className="dashboard-entry-copy"><h3>{title}</h3><p>{description}</p></div><span className="dashboard-entry-action">{index === 0 ? "开始处理" : "进入"}<ArrowRight size={16} aria-hidden="true" /></span></>;
          return to.startsWith("#") ? <a key={title} href={to} className={`dashboard-entry ${index === 0 ? "is-primary" : ""}`}>{content}</a> : <Link key={title} to={to} className={`dashboard-entry ${index === 0 ? "is-primary" : ""}`}>{content}</Link>;
        })}
      </div>
    </section>

    <div className="dashboard-work-grid">
      <DashboardPanel title="我的待办" id="dashboard-todo-title" description="数量来自当前账号获授权的接口返回范围。">
        {data.loading ? <LoadingState label="正在加载工作待办" /> : <>
          {sourceErrors.length > 0 && <div className="dashboard-warning" role="status">部分数据暂时不可用，未加载的数量以“—”显示。可刷新数据重试。</div>}
          {todos.length === 0 ? <EmptyState icon={BookOpen} title="暂无业务待办" description="学习账号可前往学习中心阅读资料与练习；当前没有分派学习待办的接口。" /> : <div className="dashboard-todo-list">{todos.map(({ title, description, to, icon: Icon, count, attention }) => {
            const content = <><span className="dashboard-todo-icon" aria-hidden="true"><Icon size={19} /></span><span className="dashboard-todo-copy"><strong>{title}</strong><span>{description}</span></span><span className={`dashboard-todo-count ${attention && count && count > 0 ? "needs-attention" : ""}`} aria-label={count === null ? "数量未加载" : `${count} 项`}>{count ?? "—"}</span><ArrowRight size={16} aria-hidden="true" /></>;
            return to.startsWith("#") ? <a key={title} href={to} className="dashboard-todo-row">{content}</a> : <Link key={title} to={to} className="dashboard-todo-row">{content}</Link>;
          })}</div>}
        </>}
      </DashboardPanel>
      <aside className="dashboard-summary" aria-labelledby="dashboard-summary-title">
        <div className="dashboard-summary-heading"><span aria-hidden="true"><ShieldCheck size={20} /></span><h2 id="dashboard-summary-title">当前工作摘要</h2></div>
        <p>{user?.display_name}，以下仅包含当前账号可见的工作范围。</p>
        {data.loading ? <p role="status">正在读取案件范围…</p> : data.cases.error ? <p role="status">{data.cases.error}</p> : data.cases.items.length > 0 ? <dl><div><dt>可访问案件</dt><dd>{data.cases.items.length} 件</dd></div><div><dt>其中进行中</dt><dd>{data.cases.items.filter((item) => item.status === "active").length} 件</dd></div></dl> : <p className="dashboard-scope-note">{caseDescription}</p>}
        <div className="dashboard-role-note"><strong>{access.commander || access.volunteer ? "安全与权限提醒" : "工作提醒"}</strong><p>{access.commander ? "线索审核仅限已加入的指挥案件。任务风险与安全提示请进入案件查看；本页没有独立安全告警统计。" : access.volunteer ? "行动前查看任务安全提示；受阻时及时反馈，不以任务数量代替风险判断。" : access.admin ? "治理权限不等于案件访问权限。案例发布前仍须完成脱敏与审核。" : access.family ? "公开进展以已确认信息为准。需要补充的信息请进入对应案件查看。" : "学习中心已接入资料与练习功能，学习权限不代表案件访问权限。"}</p></div>
      </aside>
    </div>

    {access.commander && !data.loading && data.intake.items.length > 0 && <DashboardPanel title="待受理案件摘要" id="dashboard-intake-title" description="仅展示受理前可见的基本信息；进入指挥端受理后查看详情。"><div className="dashboard-intake-list">{data.intake.items.slice(0, 3).map((item) => <Link to="/command" key={item.id}><span><strong>{item.case_code}</strong><span>地区：{item.area_hint ?? "待补充"} · 走失时间：{item.last_seen_at ?? "待补充"} · 老人年龄：{item.elder_age == null ? "待补充" : `${item.elder_age} 岁`}</span></span><span className="dashboard-status-tag">待受理</span></Link>)}</div></DashboardPanel>}

    <DashboardPanel title="最近更新" id="dashboard-recent-title" description="按返回记录的更新时间排列，最多展示5条；不是操作日志。">
      {data.loading ? <LoadingState label="正在加载最近更新" /> : recent.length > 0 ? <ul className="dashboard-recent-list">{recent.map((item) => <li key={item.key}><Clock3 size={17} aria-hidden="true" /><div>{item.to ? <Link to={item.to}>{item.title}</Link> : <strong>{item.title}</strong>}<p>{item.description}</p></div><time dateTime={item.at}>{displayTime(item.at)}</time></li>)}</ul> : data.cases.error || data.tasks.error || data.sessions.error ? <ErrorState message="最近更新的数据源暂时不可用。" onRetry={() => void data.load()} /> : <EmptyState icon={Clock3} title="暂无最近更新" description="相关案件、任务或求助登记更新后，会在这里显示。" />}
    </DashboardPanel>

    {access.admin && <section id="archive-review" className="dashboard-archive" aria-labelledby="archive-review-title">
      <header className="dashboard-panel-heading"><h2 id="archive-review-title">案例整理草稿</h2><p>保留现有审核流程：先确认脱敏，再审核发布；撤回不影响原案件。</p></header>
      {data.loading ? <LoadingState label="正在加载归档审核" /> : data.archives.error ? <ErrorState message={data.archives.error} onRetry={() => void data.load()} /> : data.archives.items.length === 0 ? <EmptyState icon={ClipboardCheck} title="暂无案例整理草稿" description="需要审核的归档材料会显示在这里。" /> : <div className="dashboard-archive-list">{data.archives.items.map((draft) => <ArchiveReviewCard key={draft.id} draft={draft} token={token} onChanged={data.updateArchive} />)}</div>}
    </section>}
  </div>;
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
  const [retentionConfirmed, setRetentionConfirmed] = useState(false);
  const [deidentifiedMaterial, setDeidentifiedMaterial] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [materials, setMaterials] = useState<ArchiveReviewMaterial[]>([]);
  const [materialError, setMaterialError] = useState("");
  const [diff, setDiff] = useState<{
    from: number;
    to: number;
    added: string[];
    removed: string[];
  } | null>(null);

  const loadMaterials = useCallback(() => {
    if (!token) return;
    void listArchiveReviewMaterials(token, draft.id)
      .then((items) => {
        setMaterials(items);
        setMaterialError("");
      })
      .catch((cause) =>
        setMaterialError(
          cause instanceof Error ? cause.message : "无法加载审核材料版本。",
        ),
      );
  }, [draft.id, token]);

  useEffect(() => {
    setDiff(null);
    loadMaterials();
  }, [loadMaterials]);

  const action = async (run: () => Promise<ArchiveDraft>) => {
    if (!token || !reason.trim()) return;
    setBusy(true);
    setError("");
    try {
      onChanged(await run());
      setDiff(null);
      loadMaterials();
      setReason("");
      setRetentionConfirmed(false);
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
      <div className="mt-3 border-t border-slate-100 pt-3">
        <div className="flex items-center justify-between gap-3">
          <strong className="text-xs text-slate-800">审核材料版本</strong>
          <span className="text-xs text-slate-500">
            {materials.length} 个版本
          </span>
        </div>
        {materials.length > 0 && (
          <div className="mt-2 space-y-2">
            {materials.map((material) => (
              <div
                key={material.id}
                className="rounded border border-slate-200 bg-slate-50 p-2 text-xs"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span>
                    v{material.version} · {material.status}
                    {material.selected_for_ai ? " · 当前 AI 输入" : ""}
                  </span>
                  {material.status === "deidentified" &&
                    !material.selected_for_ai &&
                    matchesRestorableArchiveDraftStatus(draft.status) && (
                      <Button
                        size="sm"
                        variant="ghost"
                        isDisabled={busy || !reason.trim()}
                        onPress={() => {
                          if (!token) return;
                          setBusy(true);
                          void restoreArchiveReviewMaterial(
                            token,
                            draft.id,
                            material.version,
                            reason,
                          )
                            .then((updated) => {
                              onChanged(updated);
                              setDiff(null);
                              loadMaterials();
                            })
                            .catch((cause) =>
                              setMaterialError(
                                cause instanceof Error
                                  ? cause.message
                                  : "无法恢复审核材料版本。",
                              ),
                            )
                            .finally(() => setBusy(false));
                        }}
                      >
                        恢复此版本
                      </Button>
                    )}
                </div>
                <p className="mb-0 mt-1 whitespace-pre-wrap text-slate-600">
                  {material.content}
                </p>
                <p className="mb-0 mt-1 text-slate-500">
                  Scope: {material.source_scope.join(", ")}
                </p>
                {materials.length > 1 &&
                  material.version !==
                    materials[materials.length - 1].version && (
                    <Button
                      className="mt-1"
                      size="sm"
                      variant="ghost"
                      onPress={() => {
                        if (!token) return;
                        const oldest = materials[materials.length - 1].version;
                        void diffArchiveReviewMaterials(
                          token,
                          draft.id,
                          oldest,
                          material.version,
                        )
                          .then((value) =>
                            setDiff({
                              from: value.from_version,
                              to: value.to_version,
                              added: value.added,
                              removed: value.removed,
                            }),
                          )
                          .catch((cause) =>
                            setMaterialError(
                              cause instanceof Error
                                ? cause.message
                                : "无法比较审核材料版本。",
                            ),
                          );
                      }}
                    >
                      与最早版本比较
                    </Button>
                  )}
              </div>
            ))}
          </div>
        )}
        {diff && (
          <div className="mt-2 rounded border border-violet-200 bg-violet-50 p-2 text-xs">
            <strong>
              差异：v{diff.from} 与 v{diff.to}
            </strong>
            {diff.added.length > 0 && (
              <p className="mb-0 mt-1 text-green-800">
                新增：{diff.added.join(" | ")}
              </p>
            )}
            {diff.removed.length > 0 && (
              <p className="mb-0 mt-1 text-red-800">
                删除：{diff.removed.join(" | ")}
              </p>
            )}
            {diff.added.length === 0 && diff.removed.length === 0 && (
              <p className="mb-0 mt-1">没有行级变更。</p>
            )}
          </div>
        )}
        {materialError && (
          <p className="mb-0 mt-2 text-xs text-red-700">{materialError}</p>
        )}
      </div>
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
                {!draft.reviewed_at && (
                  <label className="flex items-start gap-2 text-xs leading-5 text-slate-700">
                    <input
                      type="checkbox"
                      checked={retentionConfirmed}
                      onChange={(event) =>
                        setRetentionConfirmed(event.target.checked)
                      }
                      className="mt-1"
                    />
                    已核对原始材料的保留范围和访问限制
                  </label>
                )}
                {!draft.reviewed_at && (
                  <Button
                    size="sm"
                    variant="primary"
                    isDisabled={busy || !reason.trim() || !retentionConfirmed}
                    onPress={() =>
                      void action(() =>
                        reviewArchiveDraft(token!, draft.id, {
                          action: "approve",
                          reason,
                          confirm_retention: true,
                        }),
                      )
                    }
                  >
                    审核归档材料
                  </Button>
                )}
                {draft.reviewed_at && (
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
                    案件归档后发布新人材料
                  </Button>
                )}
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

function matchesRestorableArchiveDraftStatus(
  status: ArchiveDraft["status"],
): boolean {
  return status === "pending_review";
}
