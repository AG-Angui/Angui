import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router";
import { within } from "@testing-library/react";
import { ApiClientError } from "../api/client";

function renderDashboard() { return render(<MemoryRouter><DashboardPage /></MemoryRouter>); }
import { DashboardPage } from "./DashboardPage";

const mocked = vi.hoisted(() => ({
  deidentifyArchiveDraft: vi.fn(),
  reviewArchiveDraft: vi.fn(),
  getCase: vi.fn(),
  listAdminArchiveDrafts: vi.fn(),
  listArchiveReviewMaterials: vi.fn(),
  diffArchiveReviewMaterials: vi.fn(),
  restoreArchiveReviewMaterial: vi.fn(),
  listCases: vi.fn(),
  listCommandIntake: vi.fn(),
  listMyTasks: vi.fn(),
  listIntakeSessions: vi.fn(),
  listAccessRequests: vi.fn(),
  accountType: "member",
  globalCapabilities: [] as string[],
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({
    token: "test-session",
    user: {
      id: "family-1",
      email: "family@demo.invalid",
      display_name: "模拟家属",
      account_type: mocked.accountType,
      global_capabilities: mocked.globalCapabilities,
    },
  }),
}));
vi.mock("../api/intake", () => ({ listIntakeSessions: (...args: unknown[]) => mocked.listIntakeSessions(...args) }));
vi.mock("../api/accessRequests", () => ({ listAccessRequests: (...args: unknown[]) => mocked.listAccessRequests(...args) }));
vi.mock("../api/cases", () => ({
  listMyTasks: (...args: unknown[]) => mocked.listMyTasks(...args),
  deidentifyArchiveDraft: (...args: unknown[]) =>
    mocked.deidentifyArchiveDraft(...args),
  reviewArchiveDraft: (...args: unknown[]) =>
    mocked.reviewArchiveDraft(...args),
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  listCommandIntake: (...args: unknown[]) => mocked.listCommandIntake(...args),
  listAdminArchiveDrafts: (...args: unknown[]) =>
    mocked.listAdminArchiveDrafts(...args),
  listArchiveReviewMaterials: (...args: unknown[]) =>
    mocked.listArchiveReviewMaterials(...args),
  diffArchiveReviewMaterials: (...args: unknown[]) =>
    mocked.diffArchiveReviewMaterials(...args),
  restoreArchiveReviewMaterial: (...args: unknown[]) =>
    mocked.restoreArchiveReviewMaterial(...args),
}));
vi.mock("../components/ServiceStatus", () => ({
  ServiceStatus: () => <span>服务状态</span>,
}));

describe("DashboardPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.globalCapabilities = [];
    mocked.accountType = "member";
    mocked.listMyTasks.mockResolvedValue([]);
    mocked.listIntakeSessions.mockResolvedValue([]);
    mocked.listAccessRequests.mockResolvedValue([]);
    mocked.listCases.mockResolvedValue([]);
    mocked.deidentifyArchiveDraft.mockResolvedValue({});
    mocked.reviewArchiveDraft.mockResolvedValue({});
    mocked.listCommandIntake.mockResolvedValue([]);
    mocked.listAdminArchiveDrafts.mockResolvedValue([]);
    mocked.listArchiveReviewMaterials.mockResolvedValue([]);
    mocked.diffArchiveReviewMaterials.mockResolvedValue({
      from_version: 1,
      to_version: 2,
      added: [],
      removed: [],
    });
    mocked.restoreArchiveReviewMaterial.mockResolvedValue({});
  });

  it("discloses when the 20-detail limit truncates command review scope", async () => {
    mocked.globalCapabilities = ["commander"];
    mocked.listCases.mockResolvedValue(
      Array.from({ length: 21 }, (_, index) => ({
        id: `case-${index + 1}`,
        case_code: `AG-${index + 1}`,
        status: "active",
        access_role: "commander",
        display_name: `模拟案件 ${index + 1}`,
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      })),
    );
    mocked.getCase.mockResolvedValue({ clues: [] });

    renderDashboard();

    expect(
      await screen.findByText("仅已加载的前20个指挥成员案件，非全量统计。"),
    ).toBeInTheDocument();
    expect(mocked.getCase).toHaveBeenCalledTimes(20);
  });

  it("shows the commander intake queue in overview metrics and real-time status", async () => {
    mocked.globalCapabilities = ["commander"];
    mocked.listCases.mockResolvedValue([]);
    mocked.listCommandIntake.mockResolvedValue([
      {
        id: "pending-case",
        case_code: "AG-PENDING",
        created_at: "2026-07-24T00:00:00Z",
        last_seen_at: "2026-07-24T08:30:00Z",
        area_hint: "北门区域",
        elder_age: 76,
      },
    ]);

    renderDashboard();

    expect(await screen.findByText("AG-PENDING")).toBeInTheDocument();
    expect(screen.getByText("待受理案件")).toBeInTheDocument();
    expect(screen.getByText("AG-PENDING")).toBeInTheDocument();
    expect(
      screen.getByText(
        /地区：北门区域.*走失时间：2026-07-24T08:30:00Z.*老人年龄：76 岁/,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("待受理")).toBeInTheDocument();
    expect(mocked.listCommandIntake).toHaveBeenCalledWith("test-session");
  });

  it("prioritizes family help, resumes real drafts and never loads operational queues", async () => {
    mocked.listIntakeSessions.mockResolvedValue([{ id: "draft & one", updated_at: "2026-11-07T08:00:00Z" }]);
    renderDashboard();
    expect(await screen.findByRole("link", { name: /继续求助登记/ })).toHaveAttribute("href", "/family/intake?session_id=draft%20%26%20one");
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(screen.getByRole("heading", { name: "总览首页" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /发起求助/ })).toHaveAttribute("href", "/family/intake");
    expect(screen.getByRole("link", { name: /公开进展/ })).toHaveAttribute("href", "/family");
    expect(mocked.listMyTasks).not.toHaveBeenCalled();
    expect(mocked.listCommandIntake).not.toHaveBeenCalled();
    expect(mocked.listAdminArchiveDrafts).not.toHaveBeenCalled();
    expect(mocked.listAccessRequests).not.toHaveBeenCalled();
    expect(mocked.getCase).not.toHaveBeenCalled();
  });

  it("counts assigned and collaborator volunteer work but not terminal or unclaimed tasks", async () => {
    mocked.globalCapabilities = ["volunteer"];
    mocked.listMyTasks.mockResolvedValue(["assigned", "accepted", "active", "blocked", "completed", "cancelled", "pending_claim"].map((status) => ({ id: status, title: status, status, updated_at: "2026-11-07T08:00:00Z", assigned_volunteer_user_id: "someone-else", collaborators: [{ volunteer_user_id: "family-1" }] })));
    renderDashboard();
    const row = await screen.findByRole("link", { name: /待处理任务/ });
    expect(within(row).getByLabelText("4 项")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /我的任务.*开始处理/ })).toHaveAttribute("href", "/volunteer");
    expect(screen.getByRole("link", { name: /学习中心.*进入/ })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /发起求助/ })).not.toBeInTheDocument();
    expect(mocked.listIntakeSessions).not.toHaveBeenCalled();
  });

  it("never treats a command-capable member's family case as command review work", async () => {
    mocked.globalCapabilities = ["commander"];
    mocked.listCases.mockResolvedValue([
      { id: "family-case", case_code: "F", access_role: "family", updated_at: "2026-11-07T08:00:00Z" },
      { id: "command-case", case_code: "C", access_role: "commander", updated_at: "2026-11-07T08:00:00Z" },
    ]);
    mocked.getCase.mockResolvedValue({ id: "command-case", clues: [{ status: "pending_review" }, { status: "needs_verification" }] });
    renderDashboard();
    const row = await screen.findByRole("link", { name: /待审核线索/ });
    expect(within(row).getByLabelText("1 项")).toBeInTheDocument();
    expect(row).toHaveAttribute("href", "/command/cases/command-case#case-clues");
    expect(mocked.getCase).toHaveBeenCalledExactlyOnceWith("test-session", "command-case");
    expect(screen.queryByRole("link", { name: /学习中心/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /发起求助/ })).not.toBeInTheDocument();
  });

  it("keeps combined capabilities additive without inventing family access", async () => {
    mocked.globalCapabilities = ["commander", "volunteer"];
    renderDashboard();
    await screen.findByRole("link", { name: /待处理任务/ });
    for (const name of [/案件与线索/, /我的任务.*进入/, /学习中心/]) expect(screen.getByRole("link", { name })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /公开进展/ })).not.toBeInTheDocument();
    expect(mocked.listMyTasks).toHaveBeenCalledWith("test-session");
    expect(mocked.listCommandIntake).toHaveBeenCalledWith("test-session");
  });

  it("shows real admin outstanding counts and no case-membership shortcuts", async () => {
    mocked.globalCapabilities = ["admin"];
    mocked.listAccessRequests.mockResolvedValue(["pending_review", "pending_review", "approved", "pending_email_verification", "rejected"].map((status) => ({ status })));
    renderDashboard();
    const row = await screen.findByRole("link", { name: /待审核账号/ });
    expect(within(row).getByLabelText("2 项")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /归档审核.*进入/ })).toHaveAttribute("href", "#archive-review");
    expect(screen.queryByRole("link", { name: /案件与线索|发起求助|我的任务|学习中心/ })).not.toBeInTheDocument();
    expect(screen.getByText("管理员账号不自动拥有案件权限")).toBeInTheDocument();
  });

  it("does not manufacture a personal learning queue or fetch cases for learners", async () => {
    mocked.accountType = "learner";
    renderDashboard();
    expect(await screen.findByText("暂无业务待办")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /学习中心.*开始处理/ })).toBeInTheDocument();
    expect(screen.getByText("新人账号暂未获得案件权限")).toBeInTheDocument();
    expect(mocked.listCases).not.toHaveBeenCalled();
    expect(mocked.listIntakeSessions).not.toHaveBeenCalled();
  });

  it("isolates a failed task source and retries explicitly without displaying a false zero", async () => {
    mocked.globalCapabilities = ["commander", "volunteer"];
    mocked.listMyTasks.mockRejectedValueOnce(new Error("private-detail")).mockResolvedValueOnce([]);
    mocked.listCommandIntake.mockResolvedValue([{ id: "intake", case_code: "AG-PENDING", area_hint: null }]);
    renderDashboard();
    const row = await screen.findByRole("link", { name: /待处理任务/ });
    expect(within(row).getByLabelText("数量未加载")).toHaveTextContent("—");
    expect(within(screen.getByRole("link", { name: /待受理案件.*1 项/ })).getByLabelText("1 项")).toBeInTheDocument();
    expect(screen.queryByText("private-detail")).not.toBeInTheDocument();
    expect(screen.getByText("AG-PENDING").closest("a")).toHaveAttribute("href", "/command");
    fireEvent.click(screen.getByRole("button", { name: "刷新数据" }));
    expect(await screen.findByRole("link", { name: /待处理任务.*0 项/ })).toBeInTheDocument();
  });

  it("does not report zero pending clues when a detail request fails", async () => {
    mocked.globalCapabilities = ["commander"];
    mocked.listCases.mockResolvedValue([{ id: "case", access_role: "commander", case_code: "AG-1", updated_at: "2026-11-07T08:00:00Z" }]);
    mocked.getCase.mockRejectedValue(new Error("unavailable"));
    renderDashboard();
    const row = await screen.findByRole("link", { name: /待审核线索/ });
    expect(within(row).getByLabelText("数量未加载")).toBeInTheDocument();
    expect(screen.getByText("AG-1")).toBeInTheDocument();
  });

  it("distinguishes insufficient permission from an empty queue", async () => {
    mocked.globalCapabilities = ["volunteer"];
    mocked.listMyTasks.mockRejectedValue(new ApiClientError(403, "forbidden", "你没有执行此操作的权限。"));
    renderDashboard();
    expect(await screen.findByText("当前账号无权读取我的任务。")).toBeInTheDocument();
    expect(within(screen.getByRole("link", { name: /待处理任务/ })).getByLabelText("数量未加载")).toBeInTheDocument();
  });

  it("keeps role actions available during initial loading", () => {
    mocked.listIntakeSessions.mockImplementationOnce(() => new Promise(() => {}));
    renderDashboard();
    expect(screen.getByRole("link", { name: /发起求助/ })).toBeInTheDocument();
    expect(screen.getByText("正在加载工作待办")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新数据" })).toBeDisabled();
    expect(screen.queryByText("暂无最近更新")).not.toBeInTheDocument();
  });

  it("shows administrator material-version state for archive drafts", async () => {
    mocked.globalCapabilities = ["admin"];
    mocked.listAdminArchiveDrafts.mockResolvedValue([
      {
        id: "archive-1",
        case_id: "case-1",
        status: "pending_review",
        content: "Timeline draft",
        source_scope: ["confirmed_clue_review_material"],
        review_material_id: "material-2",
        deidentification_status: "deidentified",
        template_version: "case-archive-ai-v2",
        provider_model: null,
        version: 2,
        usage_scope: "internal_archive",
        retention_status: "retained",
        deidentified_at: "2026-07-25T08:00:00Z",
        reviewed_at: null,
        created_at: "2026-07-25T08:00:00Z",
        updated_at: "2026-07-25T08:00:00Z",
      },
    ]);
    mocked.listArchiveReviewMaterials.mockResolvedValueOnce([
      {
        id: "material-2",
        case_id: "case-1",
        version: 2,
        parent_material_id: "material-1",
        content: "Approved de-identified material",
        source_scope: ["confirmed_clue_review_material"],
        status: "deidentified",
        created_by_user_id: "admin-1",
        reviewed_by_user_id: "admin-1",
        reviewed_at: "2026-07-25T08:00:00Z",
        review_reason: "reviewed",
        created_at: "2026-07-25T08:00:00Z",
        selected_for_ai: true,
      },
      {
        id: "material-1",
        case_id: "case-1",
        version: 1,
        parent_material_id: null,
        content: "Earlier approved material",
        source_scope: ["confirmed_clue_review_material"],
        status: "deidentified",
        created_by_user_id: "admin-1",
        reviewed_by_user_id: "admin-1",
        reviewed_at: "2026-07-24T08:00:00Z",
        review_reason: "reviewed",
        created_at: "2026-07-24T08:00:00Z",
        selected_for_ai: false,
      },
    ]);
    mocked.listArchiveReviewMaterials.mockResolvedValueOnce([
      {
        id: "material-3",
        case_id: "case-1",
        version: 3,
        parent_material_id: "material-1",
        content: "Restored approved material",
        source_scope: ["confirmed_clue_review_material"],
        status: "deidentified",
        created_by_user_id: "admin-1",
        reviewed_by_user_id: "admin-1",
        reviewed_at: "2026-07-25T09:00:00Z",
        review_reason: "restored",
        created_at: "2026-07-25T09:00:00Z",
        selected_for_ai: true,
      },
      {
        id: "material-2",
        case_id: "case-1",
        version: 2,
        parent_material_id: "material-1",
        content: "Approved de-identified material",
        source_scope: ["confirmed_clue_review_material"],
        status: "deidentified",
        created_by_user_id: "admin-1",
        reviewed_by_user_id: "admin-1",
        reviewed_at: "2026-07-25T08:00:00Z",
        review_reason: "reviewed",
        created_at: "2026-07-25T08:00:00Z",
        selected_for_ai: false,
      },
      {
        id: "material-1",
        case_id: "case-1",
        version: 1,
        parent_material_id: null,
        content: "Earlier approved material",
        source_scope: ["confirmed_clue_review_material"],
        status: "deidentified",
        created_by_user_id: "admin-1",
        reviewed_by_user_id: "admin-1",
        reviewed_at: "2026-07-24T08:00:00Z",
        review_reason: "reviewed",
        created_at: "2026-07-24T08:00:00Z",
        selected_for_ai: false,
      },
    ]);
    mocked.diffArchiveReviewMaterials.mockResolvedValue({
      from_version: 1,
      to_version: 2,
      added: ["Approved de-identified material"],
      removed: ["Earlier approved material"],
    });
    mocked.restoreArchiveReviewMaterial.mockResolvedValue({
      id: "archive-1",
      case_id: "case-1",
      status: "pending_review",
      content: "Regenerated archive draft",
      source_scope: ["confirmed_clue_review_material"],
      review_material_id: "material-3",
      deidentification_status: "deidentified",
      template_version: "case-archive-ai-v2",
      provider_model: null,
      version: 3,
      usage_scope: "internal_archive",
      retention_status: "retained",
      deidentified_at: "2026-07-25T08:00:00Z",
      reviewed_at: null,
      created_at: "2026-07-25T08:00:00Z",
      updated_at: "2026-07-25T08:00:00Z",
    });

    renderDashboard();

    expect(await screen.findByText("审核材料版本")).toBeInTheDocument();
    expect(
      await screen.findByText("Approved de-identified material"),
    ).toBeInTheDocument();
    expect(screen.getByText(/当前 AI 输入/)).toBeInTheDocument();
    expect(mocked.listArchiveReviewMaterials).toHaveBeenCalledWith(
      "test-session",
      "archive-1",
    );

    fireEvent.click(screen.getByRole("button", { name: "与最早版本比较" }));
    expect(await screen.findByText("差异：v1 与 v2")).toBeInTheDocument();
    expect(mocked.diffArchiveReviewMaterials).toHaveBeenCalledWith(
      "test-session",
      "archive-1",
      1,
      2,
    );

    fireEvent.change(screen.getByLabelText(/审核理由/), {
      target: { value: "restore approved historical version" },
    });
    fireEvent.click(screen.getByRole("button", { name: "恢复此版本" }));
    expect(mocked.restoreArchiveReviewMaterial).toHaveBeenCalledWith(
      "test-session",
      "archive-1",
      1,
      "restore approved historical version",
    );
    expect(
      await screen.findByText("Restored approved material"),
    ).toBeInTheDocument();
    expect(mocked.listArchiveReviewMaterials).toHaveBeenCalledTimes(2);
  });
});
