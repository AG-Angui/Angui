import { useCallback, useEffect, useRef, useState } from "react";
import { listAccessRequests, type AdminAccessRequest } from "../api/accessRequests";
import { getCase, listAdminArchiveDrafts, listCases, listCommandIntake, listMyTasks, type ArchiveDraft, type CaseDetail, type CaseListItem, type CaseTask, type CommandIntakeCase } from "../api/cases";
import { listIntakeSessions, type IntakeSession } from "../api/intake";
import { useAuth } from "../auth/useAuth";
import { ApiClientError } from "../api/client";
import { workspaceAccess } from "../workspaceNavigation";

type Source<T> = { items: T[]; error: string; partial?: boolean };
type DashboardData = {
  cases: Source<CaseListItem>;
  intake: Source<CommandIntakeCase>;
  sessions: Source<IntakeSession>;
  tasks: Source<CaseTask>;
  archives: Source<ArchiveDraft>;
  requests: Source<AdminAccessRequest>;
  reviews: Source<CaseDetail>;
};
const empty = <T,>(): Source<T> => ({ items: [], error: "" });
const initialData = (): DashboardData => ({ cases: empty(), intake: empty(), sessions: empty(), tasks: empty(), archives: empty(), requests: empty(), reviews: empty() });

async function readSource<T>(enabled: boolean, load: () => Promise<T[]>, label: string): Promise<Source<T>> {
  if (!enabled) return empty();
  try { return { items: await load(), error: "" }; }
  catch (cause) {
    return {
      items: [],
      error: cause instanceof ApiClientError && cause.status === 403
        ? `当前账号无权读取${label}。`
        : `${label}暂时无法加载，请刷新重试。`,
    };
  }
}

export function useDashboardData() {
  const { token, user } = useAuth();
  const { family, commander, volunteer, admin } = workspaceAccess(user);
  const isMember = user?.account_type === "member";
  const [data, setData] = useState(initialData);
  const [loading, setLoading] = useState(true);
  const generation = useRef(0);
  const load = useCallback(async () => {
    const request = ++generation.current;
    if (!token) { setData(initialData()); setLoading(false); return; }
    setLoading(true);
    const [cases, intake, sessions, tasks, archives, requests] = await Promise.all([
      readSource(Boolean(isMember), () => listCases(token), "可访问案件"),
      readSource(commander, () => listCommandIntake(token), "待受理案件"),
      readSource(family, () => listIntakeSessions(token), "求助登记待办"),
      readSource(volunteer, () => listMyTasks(token), "我的任务"),
      readSource(admin, () => listAdminArchiveDrafts(token), "归档审核"),
      readSource(admin, () => listAccessRequests(token), "账号审核"),
    ]);
    if (request !== generation.current) return;
    // Case membership, not the account's global capability, determines which
    // clue records are review work. Bound fan-out and disclose the exact scope.
    const reviewable = commander ? cases.items.filter((item) => item.access_role === "commander") : [];
    const results = await Promise.allSettled(reviewable.slice(0, 20).map((item) => getCase(token, item.id)));
    const reviews: Source<CaseDetail> = {
      items: results.flatMap((result) => result.status === "fulfilled" ? [result.value] : []),
      error: cases.error && commander ? "案件列表未加载，无法读取线索待办。" : results.some((result) => result.status === "rejected") ? "部分案件详情暂时无法加载，线索待办不完整。" : "",
      partial: reviewable.length > 20,
    };
    if (request !== generation.current) return;
    setData({ cases, intake, sessions, tasks, archives, requests, reviews });
    setLoading(false);
  }, [token, isMember, family, commander, volunteer, admin]);

  useEffect(() => {
    void load();
    return () => { generation.current += 1; };
  }, [load]);

  const updateArchive = (updated: ArchiveDraft) => setData((current) => ({ ...current, archives: { ...current.archives, items: current.archives.items.map((item) => item.id === updated.id ? updated : item) } }));
  return { ...data, loading, load, updateArchive };
}
