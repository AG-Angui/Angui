import {
  act,
  fireEvent,
  render as renderUi,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { MemoryRouter, Route, Routes } from "react-router";
import type { CaseDetail, CaseRole, CaseStatus, Clue } from "../api/cases";
import { CaseWorkspacePage } from "./CaseWorkspacePage";

function render(ui: ReactElement) {
  const wrapped = (nextUi: ReactElement) => (
    <MemoryRouter initialEntries={["/command/cases/case-command"]}>
      <Routes>
        <Route path="/command/cases/:caseId" element={nextUi} />
        <Route path="*" element={nextUi} />
      </Routes>
    </MemoryRouter>
  );
  const result = renderUi(wrapped(ui));
  return { ...result, rerender: (nextUi: ReactElement) => result.rerender(wrapped(nextUi)) };
}

const mocked = vi.hoisted(() => ({
  auth: { token: "test-session" as string | null },
  getCase: vi.fn(),
  getCaseMapView: vi.fn(),
  getCasePublicProgress: vi.fn(),
  getLatestSummaryDraft: vi.fn(),
  getCaseResourceConfiguration: vi.fn(),
  listCases: vi.fn(),
  listCommandIntake: vi.fn(),
  acceptCommandCase: vi.fn(),
  listCaseClues: vi.fn(),
  listCasePois: vi.fn(),
  addCaseMember: vi.fn(),
  listCaseTasks: vi.fn(),
  listCaseMembers: vi.fn(),
  createCaseTask: vi.fn(),
  reviewClue: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({ token: mocked.auth.token }),
}));
vi.mock("../api/cases", () => ({
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  getCaseMapView: (...args: unknown[]) => mocked.getCaseMapView(...args),
  getCasePublicProgress: (...args: unknown[]) =>
    mocked.getCasePublicProgress(...args),
  getLatestSummaryDraft: (...args: unknown[]) =>
    mocked.getLatestSummaryDraft(...args),
  getCaseResourceConfiguration: (...args: unknown[]) =>
    mocked.getCaseResourceConfiguration(...args),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  listCommandIntake: (...args: unknown[]) => mocked.listCommandIntake(...args),
  acceptCommandCase: (...args: unknown[]) => mocked.acceptCommandCase(...args),
  listCaseClues: (...args: unknown[]) => mocked.listCaseClues(...args),
  listCasePois: (...args: unknown[]) => mocked.listCasePois(...args),
  addCaseMember: (...args: unknown[]) => mocked.addCaseMember(...args),
  listCaseTasks: (...args: unknown[]) => mocked.listCaseTasks(...args),
  listCaseMembers: (...args: unknown[]) => mocked.listCaseMembers(...args),
  createCaseTask: (...args: unknown[]) => mocked.createCaseTask(...args),
  createCase: vi.fn(),
  createClue: vi.fn(),
  reviewClue: (...args: unknown[]) => mocked.reviewClue(...args),
  createCasePlace: vi.fn(),
  uploadCaseAttachment: vi.fn(),
  updateCaseStatus: vi.fn(),
  updateElderProfile: vi.fn(),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

function detail(
  id: string,
  displayName: string,
  accessRole: CaseRole = "family",
  status: CaseStatus = "active",
): CaseDetail {
  return {
    id,
    case_code: `AG-${id}`,
    status,
    access_role: accessRole,
    elder_profile: {
      id: `profile-${id}`,
      display_name: displayName,
      age: null,
      gender: null,
      physical_description: null,
      clothing_description: null,
      health_notes: null,
      last_seen_at: null,
      last_seen_location: null,
    },
    clues: [],
    places: [],
    attachments: [],
    created_at: "2026-07-24T00:00:00Z",
    updated_at: "2026-07-24T00:00:00Z",
  };
}

describe("CaseWorkspacePage", () => {
  it("gives a family member one next action and keeps long forms collapsed", async () => {
    vi.clearAllMocks();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-family-workspace",
        case_code: "AG-FAMILY-WORKSPACE",
        status: "active",
        access_role: "family",
        display_name: "Family workspace",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-family-workspace", "Family workspace", "family"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCaseMapView.mockResolvedValue({ items: [] });

    render(<CaseWorkspacePage mode="family" />);

    expect(
      await screen.findByRole("heading", { name: "当前可以做什么" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "补充人物资料（主操作）" }),
    ).toHaveAttribute("href", "#case-profile-editor");
    expect(document.getElementById("case-profile-editor")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "案件工作区导航" })).toBeInTheDocument();
    expect(screen.getByText("补充或更正人物资料").closest("details")).not.toHaveAttribute(
      "open",
    );
    expect(screen.getByText("案件状态与成员管理").closest("details")).not.toHaveAttribute(
      "open",
    );
    expect(screen.getByText("提交一条新线索").closest("details")).not.toHaveAttribute(
      "open",
    );
    expect(screen.queryByRole("heading", { name: "任务看板" })).not.toBeInTheDocument();
    expect(mocked.listCaseTasks).not.toHaveBeenCalled();
  });

  it("sends a volunteer to assigned tasks from the primary action", async () => {
    vi.clearAllMocks();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-volunteer-workspace",
        case_code: "AG-VOLUNTEER-WORKSPACE",
        status: "active",
        access_role: "volunteer",
        display_name: "Volunteer workspace",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-volunteer-workspace", "Volunteer workspace", "volunteer"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCaseMapView.mockResolvedValue({ items: [] });
    mocked.listCaseTasks.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });
    mocked.listCaseMembers.mockResolvedValue([]);
    mocked.listCaseClues.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });

    render(<CaseWorkspacePage mode="volunteer" />);

    expect(await screen.findByRole("heading", { name: "协作提示" })).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "查看已分配任务（主操作）" }),
    ).toHaveAttribute("href", "#task-board");
    expect(document.getElementById("task-board")).toBeInTheDocument();
  });

  it("does not offer family clue submission after a case is resolved", async () => {
    vi.clearAllMocks();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-family-resolved",
        case_code: "AG-FAMILY-RESOLVED",
        status: "resolved",
        access_role: "family",
        display_name: "Resolved family workspace",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail(
        "case-family-resolved",
        "Resolved family workspace",
        "family",
        "resolved",
      ),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCaseMapView.mockResolvedValue({ items: [] });

    render(<CaseWorkspacePage mode="family" />);

    expect(
      await screen.findByText("案件已找到，不能再提交补充信息。"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "查看案件资料（主操作）" }),
    ).toHaveAttribute("href", "#case-profile");
    expect(document.getElementById("case-profile")).toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: "提交一条新线索（主操作）" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("提交一条新线索")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "提交线索" })).not.toBeInTheDocument();
  });

  it("puts commander task and review work ahead of case materials", async () => {
    vi.clearAllMocks();
    mocked.listCommandIntake.mockResolvedValue([]);
    mocked.listCases.mockResolvedValue([
      {
        id: "case-commander-workspace",
        case_code: "AG-COMMANDER-WORKSPACE",
        status: "active",
        access_role: "commander",
        display_name: "Commander workspace",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-commander-workspace", "Commander workspace", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCaseMapView.mockResolvedValue({ items: [] });
    mocked.listCaseTasks.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });
    mocked.listCaseMembers.mockResolvedValue([]);
    mocked.listCaseClues.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });

    render(<CaseWorkspacePage mode="commander" />);

    expect(
      await screen.findByRole("heading", { name: "指挥工作台" }),
    ).toBeInTheDocument();
    const taskHeading = screen.getByRole("heading", { name: "任务看板" });
    const clueHeading = screen.getByRole("heading", { name: "线索" });
    expect(taskHeading.compareDocumentPosition(clueHeading)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(screen.getByRole("link", { name: "前往任务和审核区（主操作）" })).toHaveAttribute(
      "href",
      "#task-board",
    );
    expect(document.getElementById("task-board")).toBeInTheDocument();
    expect(screen.getByText("案件状态与成员管理").closest("details")).not.toHaveAttribute(
      "open",
    );
  });

  it("lets a commander accept a minimal pending case before viewing it", async () => {
    mocked.listCases.mockResolvedValue([]);
    mocked.listCommandIntake.mockResolvedValue([
      {
        id: "pending-case",
        case_code: "AG-PENDING",
        created_at: "2026-07-24T00:00:00Z",
        last_seen_at: null,
        area_hint: "Fictional north gate",
        elder_age: 76,
      },
    ]);
    mocked.acceptCommandCase.mockResolvedValue(
      detail("pending-case", "Accepted case", "commander"),
    );

    renderUi(
      <MemoryRouter initialEntries={["/command"]}>
        <Routes>
          <Route path="/command" element={<CaseWorkspacePage mode="commander" />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText("AG-PENDING")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "受理案件" }));
    await waitFor(() =>
      expect(mocked.acceptCommandCase).toHaveBeenCalledWith(
        "test-session",
        "pending-case",
      ),
    );
  });

  it("renders role-filtered map records as a usable text fallback", async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "Commander case",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-command", "Commander case", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCaseMapView.mockResolvedValue({
      items: [
        {
          id: "place-text",
          object_type: "place",
          display_name: "Fictional market",
          longitude: null,
          latitude: null,
          location_text: "North gate, fictional park",
          location_precision: "unknown",
          source: "commander",
          occurred_at: null,
          reported_at: "2026-07-24T00:00:00Z",
          review_status: "confirmed",
          related_task_id: null,
          updated_at: "2026-07-24T00:00:00Z",
        },
      ],
    });
    mocked.listCaseClues.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });

    render(<CaseWorkspacePage mode="commander" />);

    expect(await screen.findByRole("navigation", { name: "案件详情导航" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "返回案件列表" })).toHaveAttribute(
      "href",
      "/command",
    );
    expect(screen.getByRole("link", { name: "任务与审核" })).toHaveAttribute(
      "href",
      "#case-tasks",
    );
    expect(screen.getByRole("link", { name: "态势与线索" })).toHaveAttribute(
      "href",
      "#case-clues",
    );
    expect(await screen.findByText("Fictional market")).toBeInTheDocument();
    expect(screen.getByText("North gate, fictional park")).toBeInTheDocument();
    expect(screen.getByText("仅文字地点")).toBeInTheDocument();
    expect(mocked.getCaseMapView).toHaveBeenCalledWith(
      "test-session",
      "case-command",
    );
  });

  it("does not let an earlier detail request overwrite the most recently selected case", async () => {
    const firstRequest = deferred<CaseDetail>();
    const secondRequest = deferred<CaseDetail>();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-1",
        case_code: "AG-1",
        status: "active",
        access_role: "family",
        display_name: "案件甲",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
      {
        id: "case-2",
        case_code: "AG-2",
        status: "active",
        access_role: "family",
        display_name: "案件乙",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockImplementation((_token: string, caseId: string) =>
      caseId === "case-1" ? firstRequest.promise : secondRequest.promise,
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });

    render(<CaseWorkspacePage mode="family" />);
    await waitFor(() =>
      expect(mocked.getCase).toHaveBeenCalledWith("test-session", "case-1"),
    );

    fireEvent.click(screen.getByText("案件乙"));
    await waitFor(() =>
      expect(mocked.getCase).toHaveBeenCalledWith("test-session", "case-2"),
    );

    await act(async () => {
      secondRequest.resolve(detail("case-2", "最新案件详情"));
      await secondRequest.promise;
    });
    expect(
      screen.getByRole("heading", { name: "最新案件详情" }),
    ).toBeInTheDocument();

    await act(async () => {
      firstRequest.reject(new Error("过期请求"));
      await firstRequest.promise.catch(() => undefined);
    });
    expect(
      screen.getByRole("heading", { name: "最新案件详情" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("过期请求")).not.toBeInTheDocument();
  });

  it("discards a stale family public-progress response after switching cases", async () => {
    vi.clearAllMocks();
    const firstProgress = deferred<Record<string, unknown>>();
    const secondProgress = deferred<Record<string, unknown>>();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-1",
        case_code: "AG-1",
        status: "active",
        access_role: "family",
        display_name: "Case one",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
      {
        id: "case-2",
        case_code: "AG-2",
        status: "active",
        access_role: "family",
        display_name: "Case two",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockImplementation((_token: string, caseId: string) =>
      Promise.resolve(
        detail(caseId, caseId === "case-1" ? "Case one" : "Case two"),
      ),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCasePublicProgress.mockImplementation(
      (_token: string, caseId: string) =>
        caseId === "case-1" ? firstProgress.promise : secondProgress.promise,
    );

    render(<CaseWorkspacePage mode="family" />);

    await screen.findByRole("heading", { name: "Case one" });
    await waitFor(() =>
      expect(mocked.getCasePublicProgress).toHaveBeenCalledWith(
        "test-session",
        "case-1",
      ),
    );
    fireEvent.click(screen.getByText("Case two"));
    await screen.findByRole("heading", { name: "Case two" });
    await waitFor(() =>
      expect(mocked.getCasePublicProgress).toHaveBeenCalledWith(
        "test-session",
        "case-2",
      ),
    );

    await act(async () => {
      secondProgress.resolve({
        case_id: "case-2",
        status: "active",
        generated_at: "2026-07-24T00:00:00Z",
        confirmed_progress: [
          {
            clue_id: "new",
            progress_type: "confirmed_update",
            review_status: "confirmed",
            updated_at: "2026-07-24T00:00:00Z",
          },
        ],
        requested_family_information: [],
        safety_and_contact_reminders: [],
      });
      await secondProgress.promise;
    });
    expect(await screen.findByText("已确认一项案件进展。")).toBeInTheDocument();

    await act(async () => {
      firstProgress.reject(new Error("stale public progress failure"));
      await firstProgress.promise.catch(() => undefined);
    });
    expect(screen.getByText("已确认一项案件进展。")).toBeInTheDocument();
    expect(
      screen.queryByText("stale public progress failure"),
    ).not.toBeInTheDocument();
  });

  it("lets an authorized commander invite the demo volunteer to an active case", async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "指挥案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-command", "指挥案件", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.addCaseMember.mockResolvedValue({});

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "指挥案件" });
    fireEvent.change(screen.getByPlaceholderText("成员邮箱"), {
      target: { value: "volunteer@demo.invalid" },
    });
    fireEvent.click(screen.getByRole("button", { name: "添加案件成员" }));

    await waitFor(() =>
      expect(mocked.addCaseMember).toHaveBeenCalledWith(
        "test-session",
        "case-command",
        "volunteer@demo.invalid",
        "volunteer",
      ),
    );
  });

  it("lets a family member invite either a family member or commander, but not a volunteer", async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: "case-family",
        case_code: "AG-FAMILY",
        status: "active",
        access_role: "family",
        display_name: "家属案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-family", "家属案件", "family"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.addCaseMember.mockResolvedValue({});

    render(<CaseWorkspacePage mode="family" />);

    await screen.findByRole("heading", { name: "家属案件" });
    const roleSelect = screen.getByRole("combobox", { name: "成员角色" });
    expect(screen.getByRole("option", { name: "家属" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "指挥" })).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "志愿者" }),
    ).not.toBeInTheDocument();

    fireEvent.change(roleSelect, { target: { value: "family" } });
    fireEvent.change(screen.getByPlaceholderText("成员邮箱"), {
      target: { value: "family-member@demo.invalid" },
    });
    fireEvent.click(screen.getByRole("button", { name: "添加案件成员" }));

    await waitFor(() =>
      expect(mocked.addCaseMember).toHaveBeenCalledWith(
        "test-session",
        "case-family",
        "family-member@demo.invalid",
        "family",
      ),
    );
  });

  it("does not expose closed-case controls that create supplementary information", async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: "case-closed",
        case_code: "AG-CLOSED",
        status: "closed",
        access_role: "commander",
        display_name: "已关闭案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-closed", "已关闭案件", "commander", "closed"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "已关闭案件" });
    expect(
      screen.queryByRole("button", { name: "提交线索" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "提交地点" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "上传图片" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "案件状态" })).toBeDisabled();
  });

  it("does not expose clue submission for a resolved case", async () => {
    mocked.listCases.mockResolvedValue([
      {
        id: "case-resolved",
        case_code: "AG-RESOLVED",
        status: "resolved",
        access_role: "commander",
        display_name: "已找到案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-resolved", "已找到案件", "commander", "resolved"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "已找到案件" });
    expect(
      screen.queryByRole("button", { name: "提交线索" }),
    ).not.toBeInTheDocument();
  });

  it("clears nearby resource results when the selected category changes", async () => {
    vi.clearAllMocks();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "Commander case",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-command", "Commander case", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.listCaseClues.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });
    mocked.listCasePois.mockResolvedValue({
      items: [
        {
          id: "hospital-1",
          name: "Fictional hospital",
          category: "hospital",
          address: "Fictional address",
          longitude: null,
          latitude: null,
        },
      ],
      source: "fixed_demo_fallback",
      degradation_status: "degraded",
      fallback_message: null,
    });

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "Commander case" });
    fireEvent.click(screen.getByRole("button", { name: "查询" }));
    expect(await screen.findByText("Fictional hospital")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("周边资源类别"), {
      target: { value: "police" },
    });
    expect(screen.queryByText("Fictional hospital")).not.toBeInTheDocument();
  });

  it("loads a filtered commander queue and requires confirmation before reviewing a clue", async () => {
    vi.clearAllMocks();
    const pendingClue = {
      id: "clue-pending",
      case_id: "case-command",
      status: "pending_review",
      source: "field responder",
      source_type: "field_report",
      content: "A fictional field observation.",
      raw_record_reference: null,
      occurred_at: null,
      reported_at: "2026-07-24T00:00:00Z",
      confirmed_at: null,
      location_text: null,
      location_precision: null,
      next_action: null,
      linked_task_reference: null,
      related_clue_id: null,
      relationship_type: null,
      review_reason: null,
      attachment_ids: [],
      created_at: "2026-07-24T00:00:00Z",
      updated_at: "2026-07-24T00:00:00Z",
      reviewed_at: null,
      is_own_submission: false,
    };
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "指挥案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-command", "指挥案件", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.listCaseClues.mockResolvedValue({
      items: [pendingClue],
      page: 1,
      page_size: 25,
      total: 1,
    });
    mocked.reviewClue.mockResolvedValue({
      ...pendingClue,
      status: "confirmed",
    });

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "指挥案件" });
    await waitFor(() =>
      expect(mocked.listCaseClues).toHaveBeenCalledWith(
        "test-session",
        "case-command",
        expect.objectContaining({
          status: "pending_review",
          sort: "created_at",
          order: "desc",
        }),
      ),
    );
    fireEvent.change(screen.getByLabelText("来源类型筛选"), {
      target: { value: "field_report" },
    });
    await waitFor(() =>
      expect(mocked.listCaseClues).toHaveBeenLastCalledWith(
        "test-session",
        "case-command",
        expect.objectContaining({ source_type: "field_report" }),
      ),
    );

    fireEvent.change(screen.getByLabelText("审核理由"), {
      target: { value: "Reviewed against the fictional record." },
    });
    const reviewTrigger = screen.getByRole("button", { name: "确认" });
    reviewTrigger.focus();
    fireEvent.click(reviewTrigger);
    expect(mocked.reviewClue).not.toHaveBeenCalled();
    const confirmationDialog = screen.getByRole("dialog", {
      name: "确认审核操作",
    });
    await waitFor(() =>
      expect(confirmationDialog.contains(document.activeElement)).toBe(true),
    );
    const cancelReview = screen.getByRole("button", { name: "取消" });
    const submitReview = screen.getByRole("button", { name: "确认提交" });
    cancelReview.focus();
    fireEvent.keyDown(cancelReview, { key: "Tab" });
    expect(submitReview).toHaveFocus();
    fireEvent.keyDown(submitReview, { key: "Tab" });
    expect(cancelReview).toHaveFocus();
    fireEvent.keyDown(confirmationDialog, { key: "Escape" });
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", { name: "确认审核操作" }),
      ).not.toBeInTheDocument(),
    );
    expect(reviewTrigger).toHaveFocus();

    fireEvent.click(reviewTrigger);
    fireEvent.click(screen.getByRole("button", { name: "确认提交" }));
    await waitFor(() =>
      expect(mocked.reviewClue).toHaveBeenCalledWith(
        "test-session",
        "clue-pending",
        expect.objectContaining({
          status: "confirmed",
          reason: "Reviewed against the fictional record.",
        }),
      ),
    );
  });

  it("describes task creation as open until a volunteer is selected", async () => {
    vi.clearAllMocks();
    const confirmedClue: Clue = {
      id: "clue-confirmed",
      case_id: "case-command",
      status: "confirmed",
      source: "commander",
      source_type: "manual_report",
      content: "Confirmed sighting for task creation.",
      raw_record_reference: null,
      occurred_at: null,
      reported_at: "2026-07-24T00:00:00Z",
      confirmed_at: "2026-07-24T01:00:00Z",
      location_text: null,
      location_precision: null,
      next_action: null,
      linked_task_reference: null,
      related_clue_id: null,
      relationship_type: null,
      review_reason: null,
      attachment_ids: [],
      created_at: "2026-07-24T00:00:00Z",
      updated_at: "2026-07-24T00:00:00Z",
      reviewed_at: "2026-07-24T01:00:00Z",
      is_own_submission: false,
    };
    const commandDetail = detail("case-command", "Commander case", "commander");
    commandDetail.clues = [confirmedClue];
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "Commander case",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(commandDetail);
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.listCaseTasks.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });
    mocked.listCaseMembers.mockResolvedValue([
      {
        user_id: "volunteer-1",
        email: "volunteer@demo.invalid",
        display_name: "Demo volunteer",
        account_type: "member",
        global_capabilities: [],
        case_role: "volunteer",
      },
    ]);

    render(<CaseWorkspacePage mode="commander" />);

    expect(
      await screen.findByRole("heading", { name: "创建开放任务" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "创建并等待志愿者申请" }),
    ).toBeInTheDocument();
    await screen.findByRole("option", { name: "Demo volunteer" });

    fireEvent.change(screen.getByLabelText("志愿者"), {
      target: { value: "volunteer-1" },
    });

    expect(
      await screen.findByRole("heading", { name: "人工创建并分配任务" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "创建并分配" }),
    ).toBeInTheDocument();
  });

  it("selects a related clue by searchable case content instead of requiring a UUID", async () => {
    vi.clearAllMocks();
    const pendingClue: Clue = {
      id: "clue-pending",
      case_id: "case-command",
      status: "pending_review",
      source: "field responder",
      source_type: "field_report",
      content: "A fictional field observation.",
      raw_record_reference: null,
      occurred_at: null,
      reported_at: "2026-07-24T00:00:00Z",
      confirmed_at: null,
      location_text: null,
      location_precision: null,
      next_action: null,
      linked_task_reference: null,
      related_clue_id: null,
      relationship_type: null,
      review_reason: null,
      attachment_ids: [],
      created_at: "2026-07-24T00:00:00Z",
      updated_at: "2026-07-24T00:00:00Z",
      reviewed_at: null,
      is_own_submission: false,
    };
    const relatedClue: Clue = {
      ...pendingClue,
      id: "clue-related",
      status: "confirmed",
      content: "公交站旁的目击记录。",
      location_text: "北门公交站",
    };
    const commandDetail = detail("case-command", "指挥案件", "commander");
    commandDetail.clues = [pendingClue, relatedClue];
    mocked.listCommandIntake.mockResolvedValue([]);
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "指挥案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(commandDetail);
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.getCaseMapView.mockResolvedValue({ items: [] });
    mocked.listCaseClues.mockResolvedValue({
      items: [pendingClue],
      page: 1,
      page_size: 25,
      total: 1,
    });
    mocked.reviewClue.mockResolvedValue({
      ...pendingClue,
      status: "duplicate",
      related_clue_id: relatedClue.id,
      relationship_type: "duplicate_of",
    });

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "指挥案件" });
    fireEvent.change(await screen.findByLabelText("审核理由"), {
      target: { value: "与已有目击记录重复。" },
    });
    fireEvent.change(screen.getByLabelText("搜索关联线索"), {
      target: { value: "公交站" },
    });
    expect(screen.getByLabelText("选择关联线索")).toHaveTextContent(
      "公交站旁的目击记录",
    );
    expect(screen.queryByText("clue-related")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("选择关联线索"), {
      target: { value: relatedClue.id },
    });
    const duplicate = screen.getByRole("button", { name: "重复" });
    expect(duplicate).toBeEnabled();
    fireEvent.click(duplicate);
    fireEvent.click(screen.getByRole("button", { name: "确认提交" }));
    await waitFor(() =>
      expect(mocked.reviewClue).toHaveBeenCalledWith(
        "test-session",
        "clue-pending",
        expect.objectContaining({
          status: "duplicate",
          related_clue_id: relatedClue.id,
          relationship_type: "duplicate_of",
        }),
      ),
    );
  });

  it("discards a stale commander queue response after filters change", async () => {
    vi.clearAllMocks();
    const firstQueue = deferred<{
      items: Array<Record<string, unknown>>;
      page: number;
      page_size: number;
      total: number;
    }>();
    const secondQueue = deferred<{
      items: Array<Record<string, unknown>>;
      page: number;
      page_size: number;
      total: number;
    }>();
    const clue = (id: string, content: string, sourceType: string) => ({
      id,
      case_id: "case-command",
      status: "pending_review",
      source: "field responder",
      source_type: sourceType,
      content,
      raw_record_reference: null,
      occurred_at: null,
      reported_at: "2026-07-24T00:00:00Z",
      confirmed_at: null,
      location_text: null,
      location_precision: null,
      next_action: null,
      linked_task_reference: null,
      related_clue_id: null,
      relationship_type: null,
      review_reason: null,
      attachment_ids: [],
      created_at: "2026-07-24T00:00:00Z",
      updated_at: "2026-07-24T00:00:00Z",
      reviewed_at: null,
      is_own_submission: false,
    });
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "指挥案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-command", "指挥案件", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.listCaseClues
      .mockReturnValueOnce(firstQueue.promise)
      .mockReturnValueOnce(secondQueue.promise);

    render(<CaseWorkspacePage mode="commander" />);

    await screen.findByRole("heading", { name: "指挥案件" });
    await waitFor(() => expect(mocked.listCaseClues).toHaveBeenCalledTimes(1));
    fireEvent.change(screen.getByLabelText("来源类型筛选"), {
      target: { value: "field_report" },
    });
    await waitFor(() => expect(mocked.listCaseClues).toHaveBeenCalledTimes(2));

    await act(async () => {
      secondQueue.resolve({
        items: [clue("latest-clue", "latest clue", "field_report")],
        page: 1,
        page_size: 25,
        total: 1,
      });
      await secondQueue.promise;
    });
    expect(await screen.findByText("latest clue")).toBeInTheDocument();

    await act(async () => {
      firstQueue.resolve({
        items: [clue("stale-clue", "stale clue", "manual_report")],
        page: 1,
        page_size: 25,
        total: 1,
      });
      await firstQueue.promise;
    });
    expect(screen.getByText("latest clue")).toBeInTheDocument();
    expect(screen.queryByText("stale clue")).not.toBeInTheDocument();
  });

  it("clears the commander queue when authentication is lost", async () => {
    vi.clearAllMocks();
    mocked.auth.token = "test-session";
    const sensitiveClue = {
      id: "sensitive-clue",
      case_id: "case-command",
      status: "pending_review",
      source: "field responder",
      source_type: "field_report",
      content: "Sensitive commander queue clue",
      raw_record_reference: null,
      occurred_at: null,
      reported_at: "2026-07-24T00:00:00Z",
      confirmed_at: null,
      location_text: null,
      location_precision: null,
      next_action: null,
      linked_task_reference: null,
      related_clue_id: null,
      relationship_type: null,
      review_reason: null,
      attachment_ids: [],
      created_at: "2026-07-24T00:00:00Z",
      updated_at: "2026-07-24T00:00:00Z",
      reviewed_at: null,
      is_own_submission: false,
    };
    mocked.listCases.mockResolvedValue([
      {
        id: "case-command",
        case_code: "AG-COMMAND",
        status: "active",
        access_role: "commander",
        display_name: "指挥案件",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      },
    ]);
    mocked.getCase.mockResolvedValue(
      detail("case-command", "指挥案件", "commander"),
    );
    mocked.getCaseResourceConfiguration.mockResolvedValue({
      attachment_max_image_bytes: 5 * 1024 * 1024,
      attachment_max_per_case: 12,
      case_place_types: ["frequent"],
    });
    mocked.listCaseClues.mockResolvedValue({
      items: [sensitiveClue],
      page: 1,
      page_size: 25,
      total: 1,
    });

    const { rerender } = render(<CaseWorkspacePage mode="commander" />);

    expect(
      await screen.findByText("Sensitive commander queue clue"),
    ).toBeInTheDocument();
    mocked.auth.token = null;
    rerender(<CaseWorkspacePage mode="commander" />);

    await waitFor(() =>
      expect(
        screen.queryByText("Sensitive commander queue clue"),
      ).not.toBeInTheDocument(),
    );
    expect(screen.queryByText("正在加载审核队列")).not.toBeInTheDocument();
    mocked.auth.token = "test-session";
  });
});
