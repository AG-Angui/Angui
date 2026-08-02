import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
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
  globalCapabilities: [] as string[],
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({
    token: "test-session",
    user: {
      id: "family-1",
      email: "family@demo.invalid",
      display_name: "模拟家属",
      account_type: "member",
      global_capabilities: mocked.globalCapabilities,
    },
  }),
}));
vi.mock("../api/cases", () => ({
  deidentifyArchiveDraft: (...args: unknown[]) =>
    mocked.deidentifyArchiveDraft(...args),
  reviewArchiveDraft: (...args: unknown[]) => mocked.reviewArchiveDraft(...args),
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  listCommandIntake: (...args: unknown[]) => mocked.listCommandIntake(...args),
  listAdminArchiveDrafts: (...args: unknown[]) => mocked.listAdminArchiveDrafts(...args),
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
    mocked.globalCapabilities = [];
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

  it("warns when the 20-detail statistics limit truncates the visible case list", async () => {
    mocked.listCases.mockResolvedValue(
      Array.from({ length: 21 }, (_, index) => ({
        id: `case-${index + 1}`,
        case_code: `AG-${index + 1}`,
        status: "active",
        access_role: "family",
        display_name: `模拟案件 ${index + 1}`,
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
      })),
    );
    mocked.getCase.mockResolvedValue({ clues: [] });

    render(<DashboardPage />);

    expect(
      await screen.findByText("部分案件详情暂时不可用，统计数据可能不完整。"),
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

    render(<DashboardPage />);

    expect(await screen.findByText("待受理案件")).toBeInTheDocument();
    expect(screen.getByText("AG-PENDING")).toBeInTheDocument();
    expect(
      screen.getByText(
        /地区：北门区域.*走失时间：2026-07-24T08:30:00Z.*老人年龄：76 岁/,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("待受理")).toBeInTheDocument();
    expect(mocked.listCommandIntake).toHaveBeenCalledWith("test-session");
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

    render(<DashboardPage />);

    expect(await screen.findByText("审核材料版本")).toBeInTheDocument();
    expect(
      await screen.findByText("Approved de-identified material"),
    ).toBeInTheDocument();
    expect(screen.getByText(/当前 AI 输入/)).toBeInTheDocument();
    expect(mocked.listArchiveReviewMaterials).toHaveBeenCalledWith("test-session", "archive-1");

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
    expect(await screen.findByText("Restored approved material")).toBeInTheDocument();
    expect(mocked.listArchiveReviewMaterials).toHaveBeenCalledTimes(2);
  });
});
