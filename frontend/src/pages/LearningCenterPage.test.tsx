import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { LearningCenterPage } from "./LearningCenterPage";

const mocked = vi.hoisted(() => ({
  getPublicPreventionCard: vi.fn(),
  askKnowledge: vi.fn(),
  listLearningQuestions: vi.fn(),
  listLearningCategories: vi.fn(),
  listLearningResources: vi.fn(),
  submitLearningCategoryProposal: vi.fn(),
  submitLearningResourceDraft: vi.fn(),
  token: "learner-session" as string | null,
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({
    token: mocked.token,
    user: mocked.token
      ? { account_type: "learner", id: "learner-1" }
      : undefined,
  }),
}));

vi.mock("../api/learning", () => ({
  getPublicPreventionCard: (...args: unknown[]) =>
    mocked.getPublicPreventionCard(...args),
  listLearningQuestions: (...args: unknown[]) =>
    mocked.listLearningQuestions(...args),
  listLearningResources: (...args: unknown[]) =>
    mocked.listLearningResources(...args),
  listLearningCategories: (...args: unknown[]) =>
    mocked.listLearningCategories(...args),
  submitLearningCategoryProposal: (...args: unknown[]) =>
    mocked.submitLearningCategoryProposal(...args),
  submitLearningResourceDraft: (...args: unknown[]) =>
    mocked.submitLearningResourceDraft(...args),
  askKnowledge: (...args: unknown[]) => mocked.askKnowledge(...args),
  submitLearningAnswer: vi.fn(),
}));

describe("LearningCenterPage", () => {
  beforeEach(() => {
    mocked.token = "learner-session";
    mocked.listLearningResources.mockResolvedValue([]);
    mocked.listLearningQuestions.mockResolvedValue([]);
    mocked.listLearningCategories.mockResolvedValue([]);
    mocked.submitLearningCategoryProposal.mockResolvedValue({
      id: "category-new",
      name: "新分类",
      status: "pending",
    });
    mocked.submitLearningResourceDraft.mockResolvedValue({});
  });

  it("renders only the approved public prevention card supplied by the API", async () => {
    mocked.getPublicPreventionCard.mockResolvedValue({
      id: "approved-card",
      title: "已审核防走失知识卡",
      summary: "摘要",
      content: "仅用于验证已审核卡片的显示。",
      resource_type: "prevention",
      tags: ["防走失"],
      source_name: "指定负责人",
      source_url: null,
      version: 2,
      effective_at: "2026-08-04T00:00:00.000Z",
    });

    render(<LearningCenterPage />);

    expect(await screen.findByText("已审核防走失知识卡")).toBeInTheDocument();
    expect(screen.getByText("仅可在线查看")).toBeInTheDocument();
    expect(
      screen.getByText("离线缓存尚未就绪，请保持联网查看。"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("来源：指定负责人 · v2", { exact: false }),
    ).toBeInTheDocument();
  });

  it("shows a Chinese waiting state when no approved prevention card exists", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "未找到可访问的资源。"),
    );

    render(<LearningCenterPage />);

    expect(await screen.findByText("等待发布")).toBeInTheDocument();
    expect(
      screen.getByText(
        "负责人尚未发布可离线使用的防走失知识卡。该卡发布并加载成功后，生产环境会保留最后一个已审核版本供离线查看。",
      ),
    ).toBeInTheDocument();
  });

  it("shows a recoverable message when the session is unavailable", async () => {
    mocked.token = null;

    render(<LearningCenterPage />);

    expect(
      await screen.findByText("登录状态不可用，请重新登录后访问学习中心。"),
    ).toBeInTheDocument();
  });

  it("labels visible resources and source-backed answers as approved published material", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "未找到可访问的资源。"),
    );
    mocked.listLearningResources.mockResolvedValue([
      {
        id: "case-study-v2",
        title: "脱敏案例复盘",
        summary: "经审核的案例摘要",
        content: "仅包含可用于培训的脱敏内容。",
        resource_type: "case_study",
        tags: ["复盘"],
        source_name: "资料负责人",
        source_url: null,
        version: 2,
        effective_at: "2026-08-05T00:00:00.000Z",
      },
    ]);
    mocked.askKnowledge.mockResolvedValue({
      answer: "请核对经审核的案例摘要。",
      certainty: "source_backed",
      sources: [
        { resource_id: "case-study-v2", title: "脱敏案例复盘", version: 2 },
      ],
      human_review_notice: "现场行动以负责人指令为准。",
    });

    render(<LearningCenterPage />);

    expect(await screen.findByText("脱敏案例")).toBeInTheDocument();
    expect(
      screen.getByText("审核状态：已发布", { exact: false }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("生效时间：", { exact: false }),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("输入学习问题"), {
      target: { value: "如何复盘？" },
    });
    fireEvent.click(screen.getByRole("button", { name: "提交问题" }));

    expect(
      await screen.findByText("资料状态：已审核资料支持"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("引用来源（均为已审核发布）：脱敏案例复盘 v2"),
    ).toBeInTheDocument();
  });

  it("keeps published resources visible when the knowledge service fails", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "未找到可访问的资源。"),
    );
    mocked.listLearningResources.mockResolvedValue([
      {
        id: "manual-v1",
        title: "已发布手册",
        summary: "摘要",
        content: "受控资料正文。",
        resource_type: "manual",
        tags: [],
        source_name: "资料负责人",
        source_url: null,
        version: 1,
        effective_at: "2026-08-05T00:00:00.000Z",
      },
    ]);
    mocked.askKnowledge.mockRejectedValue(new Error("服务暂不可用"));

    render(<LearningCenterPage />);
    expect(await screen.findByText("已发布手册")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("输入学习问题"), {
      target: { value: "如何准备？" },
    });
    fireEvent.click(screen.getByRole("button", { name: "提交问题" }));

    expect(
      await screen.findByText(
        "问答暂时不可用，已发布资料仍可查看。请稍后重试。",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("已发布手册")).toBeInTheDocument();
  });

  it("renders category and tag chips and sends server-side filter changes", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.listLearningCategories.mockResolvedValue([
      { id: "cat-safety", name: "安全基础", status: "enabled" },
    ]);
    mocked.listLearningResources.mockResolvedValue([
      {
        id: "resource-safety",
        title: "安全手册",
        summary: "摘要",
        content: "正文",
        resource_type: "manual",
        tags: ["基础"],
        category: { id: "cat-safety", name: "安全基础", status: "assigned" },
        source_name: "来源",
        source_url: null,
        version: 1,
        effective_at: "2026-08-05T00:00:00.000Z",
      },
    ]);

    render(<LearningCenterPage />);
    expect(await screen.findByText("安全手册")).toBeInTheDocument();
    expect(screen.getAllByText("安全基础").length).toBeGreaterThan(0);
    expect(screen.getByText("#基础")).toBeInTheDocument();

    mocked.listLearningResources.mockClear();
    fireEvent.change(screen.getByLabelText("分类筛选"), {
      target: { value: "cat-safety" },
    });
    await waitFor(() =>
      expect(mocked.listLearningResources).toHaveBeenLastCalledWith(
        "learner-session",
        { category_id: "cat-safety", tag: "" },
      ),
    );

    mocked.listLearningResources.mockClear();
    fireEvent.change(screen.getByLabelText("标签筛选"), {
      target: { value: "基础" },
    });
    await waitFor(() =>
      expect(mocked.listLearningResources).toHaveBeenLastCalledWith(
        "learner-session",
        { category_id: "cat-safety", tag: "基础" },
      ),
    );
  });

  it("submits learner drafts with normalized tags and fixed learner training scope", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.listLearningCategories.mockResolvedValue([
      { id: "cat-safety", name: "安全基础", status: "enabled" },
    ]);
    render(<LearningCenterPage />);

    await screen.findByRole("heading", { name: "提交学习资源草稿" });
    fireEvent.change(screen.getByLabelText("草稿标题"), {
      target: { value: "新人安全提示" },
    });
    fireEvent.change(screen.getByLabelText("草稿来源名称"), {
      target: { value: "学习小组" },
    });
    fireEvent.change(screen.getByLabelText("草稿标签"), {
      target: { value: "基础, 安全，基础" },
    });
    fireEvent.change(screen.getByLabelText("摘要"), {
      target: { value: "摘要" },
    });
    fireEvent.change(screen.getByLabelText("正文"), {
      target: { value: "正文" },
    });
    fireEvent.change(screen.getByLabelText("提交理由"), {
      target: { value: "供新人学习" },
    });
    const draftSection = screen
      .getByRole("heading", { name: "提交学习资源草稿" })
      .closest("section");
    expect(draftSection).not.toBeNull();
    fireEvent.change(draftSection!.querySelector("select")!, {
      target: { value: "cat-safety" },
    });
    fireEvent.click(screen.getByRole("button", { name: "提交草稿" }));

    await waitFor(() => expect(mocked.submitLearningResourceDraft).toHaveBeenCalled());
    const [, input] = mocked.submitLearningResourceDraft.mock.calls.at(-1)!;
    expect(input).toMatchObject({
      title: "新人安全提示",
      category_id: "cat-safety",
      tags: ["基础", "安全", "基础"],
      visibility: "learner",
      permitted_use: "training",
    });
  });

  it("shows a retryable error and re-enables the learner draft submission after failure", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.submitLearningResourceDraft.mockRejectedValue(
      new ApiClientError(503, "request_failed", "草稿服务暂时不可用"),
    );
    render(<LearningCenterPage />);

    await screen.findByRole("heading", { name: "提交学习资源草稿" });
    fireEvent.change(screen.getByLabelText("草稿标题"), {
      target: { value: "失败草稿" },
    });
    fireEvent.change(screen.getByLabelText("草稿来源名称"), {
      target: { value: "学习小组" },
    });
    fireEvent.change(screen.getByLabelText("摘要"), {
      target: { value: "摘要" },
    });
    fireEvent.change(screen.getByLabelText("正文"), {
      target: { value: "正文" },
    });
    fireEvent.change(screen.getByLabelText("提交理由"), {
      target: { value: "覆盖失败后的重试路径" },
    });
    fireEvent.click(screen.getByRole("button", { name: "提交草稿" }));

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("草稿服务暂时不可用"),
    );
    expect(screen.getByRole("button", { name: "提交草稿" })).toBeEnabled();
  });

  it("submits a learner category proposal with a reason", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    render(<LearningCenterPage />);

    await screen.findByRole("heading", { name: "提交学习资源草稿" });
    fireEvent.change(screen.getByLabelText("申请分类名称"), {
      target: { value: "现场沟通" },
    });
    fireEvent.change(screen.getByLabelText("申请分类理由"), {
      target: { value: "便于新人按场景学习" },
    });
    fireEvent.click(screen.getByRole("button", { name: "申请分类" }));

    await waitFor(() =>
      expect(mocked.submitLearningCategoryProposal).toHaveBeenCalledWith(
        "learner-session",
        "现场沟通",
        "便于新人按场景学习",
      ),
    );
    expect(screen.getByRole("status")).toHaveTextContent("分类申请已提交");
  });

  it("shows a visible error when a learner category proposal fails", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.submitLearningCategoryProposal.mockRejectedValue(
      new ApiClientError(503, "request_failed", "分类申请服务暂时不可用"),
    );
    render(<LearningCenterPage />);

    await screen.findByRole("heading", { name: "提交学习资源草稿" });
    fireEvent.change(screen.getByLabelText("申请分类名称"), {
      target: { value: "现场沟通" },
    });
    fireEvent.change(screen.getByLabelText("申请分类理由"), {
      target: { value: "覆盖失败提示" },
    });
    fireEvent.click(screen.getByRole("button", { name: "申请分类" }));

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "分类申请服务暂时不可用",
      ),
    );
  });
});
