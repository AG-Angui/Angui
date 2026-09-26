import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { LearningCenterPage } from "./LearningCenterPage";

const mocked = vi.hoisted(() => ({
  getPublicPreventionCard: vi.fn(),
  listLearningQuestions: vi.fn(),
  listLearningCategories: vi.fn(),
  listLearningResources: vi.fn(),
  listLearningAnswers: vi.fn(),
  listWrongLearningAnswers: vi.fn(),
  getLearningProgress: vi.fn(),
  submitLearningCategoryProposal: vi.fn(),
  submitLearningResourceDraft: vi.fn(),
  submitLearningAnswer: vi.fn(),
  searchLearningKnowledge: vi.fn(),
  askKnowledge: vi.fn(),
  downloadKnowledgeImage: vi.fn(),
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
  listLearningAnswers: (...args: unknown[]) => mocked.listLearningAnswers(...args),
  listWrongLearningAnswers: (...args: unknown[]) => mocked.listWrongLearningAnswers(...args),
  getLearningProgress: (...args: unknown[]) => mocked.getLearningProgress(...args),
  listLearningCategories: (...args: unknown[]) =>
    mocked.listLearningCategories(...args),
  submitLearningCategoryProposal: (...args: unknown[]) =>
    mocked.submitLearningCategoryProposal(...args),
  submitLearningResourceDraft: (...args: unknown[]) =>
    mocked.submitLearningResourceDraft(...args),
  submitLearningAnswer: (...args: unknown[]) => mocked.submitLearningAnswer(...args),
  searchLearningKnowledge: (...args: unknown[]) => mocked.searchLearningKnowledge(...args),
  askKnowledge: (...args: unknown[]) => mocked.askKnowledge(...args),
  downloadKnowledgeImage: (...args: unknown[]) => mocked.downloadKnowledgeImage(...args),
}));

describe("LearningCenterPage", () => {
  beforeEach(() => {
    localStorage.clear();
    mocked.token = "learner-session";
    mocked.listLearningResources.mockResolvedValue([]);
    mocked.listLearningAnswers.mockResolvedValue([]);
    mocked.listWrongLearningAnswers.mockResolvedValue([]);
    mocked.getLearningProgress.mockResolvedValue({ answered_questions: 0, total_answers: 0, correct_answers: 0, accuracy: 0, latest_answered_at: null });
    mocked.listLearningQuestions.mockResolvedValue([]);
    mocked.listLearningCategories.mockResolvedValue([]);
    mocked.submitLearningCategoryProposal.mockResolvedValue({
      id: "category-new",
      name: "新分类",
      status: "pending",
    });
    mocked.submitLearningResourceDraft.mockResolvedValue({});
    mocked.submitLearningAnswer.mockResolvedValue({
      question_id: "question-1",
      is_correct: true,
      score: 1,
      max_score: 1,
      explanation: "解析只会在提交后出现。",
      source: { resource_id: "resource-1", title: "来源资料", version: 1 },
    });
    mocked.searchLearningKnowledge.mockResolvedValue({ results: [] });
    mocked.askKnowledge.mockResolvedValue({ answer: "资料不足", certainty: "insufficient_sources", sources: [], human_review_notice: "请咨询负责人。" });
    mocked.downloadKnowledgeImage.mockResolvedValue(new Blob(["image"], { type: "image/png" }));
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

  it("labels visible resources as approved material while retaining governed knowledge Q&A", async () => {
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
    render(<LearningCenterPage />);

    expect(await screen.findByText("脱敏案例")).toBeInTheDocument();
    expect(
      screen.getByText("审核状态：已发布", { exact: false }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("生效时间：", { exact: false }),
    ).toBeInTheDocument();

    expect(screen.getByRole("heading", { name: "知识检索与问答" })).toBeInTheDocument();
    expect(screen.getByText("提交前不会显示答案或解析。完成作答后可查看解析与关联的已发布资料。")).toBeInTheDocument();
  });

  it("keeps published resources visible without a knowledge chat dependency", async () => {
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
    render(<LearningCenterPage />);
    expect(await screen.findByText("已发布手册")).toBeInTheDocument();
    expect(screen.getByText("已发布手册")).toBeInTheDocument();
  });

  it("loads protected knowledge images into a learner-visible preview", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.searchLearningKnowledge.mockResolvedValue({
      results: [{
        knowledge_item_id: "knowledge-1",
        title: "带附图的知识卡",
        summary: "图片应供有权限的学习者查看。",
        content: "正文",
        category: "基础",
        keywords: ["附图"],
        score: 1,
        knowledge_base_id: "learning-materials",
        version: 1,
        source_name: "管理员",
        source_url: null,
        status: "published",
        attachments: [],
        images: [{ id: "image-1", storage_path: "private/image-1.png", mime_type: "image/png", width: 20, height: 10, metadata: {} }],
      }],
    });
    const createObjectUrl = vi.fn(() => "blob:knowledge-image");
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: createObjectUrl,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });

    render(<LearningCenterPage />);
    await screen.findByRole("heading", { name: "知识检索与问答" });
    fireEvent.change(screen.getByLabelText("搜索知识"), { target: { value: "附图" } });
    fireEvent.click(screen.getByRole("button", { name: "搜索" }));

    expect(await screen.findByRole("img", { name: "带附图的知识卡 附图 1" })).toHaveAttribute(
      "src",
      "blob:knowledge-image",
    );
    expect(mocked.downloadKnowledgeImage).toHaveBeenCalledWith(
      "learner-session",
      "knowledge-1",
      "image-1",
      expect.objectContaining({ signal: expect.any(AbortSignal) }),
    );
  });

  it("offers knowledge filters before the first query and supports filter-only search", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(new ApiClientError(404, "not_found", "没有卡片"));
    mocked.searchLearningKnowledge.mockResolvedValue({
      results: [{
        knowledge_item_id: "knowledge-filter", knowledge_base_id: "learning-materials",
        title: "安全手册", summary: "摘要", content: "正文", category: "安全",
        keywords: ["核实"], matched_fields: [], score: 0, version: 1,
        source_name: "管理员", source_url: null, status: "published", attachments: [], images: [],
      }],
    });
    render(<LearningCenterPage />);
    await screen.findByRole("heading", { name: "知识检索与问答" });
    expect(await screen.findByRole("option", { name: "安全" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "核实" })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("知识分类筛选"), { target: { value: "安全" } });
    fireEvent.click(screen.getByRole("button", { name: "搜索" }));
    await waitFor(() => expect(mocked.searchLearningKnowledge).toHaveBeenLastCalledWith(
      "learner-session", { query: "", category_id: "安全", tag: "" },
    ));
  });

  it("labels a gateway degradation as source text instead of model output", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.askKnowledge.mockResolvedValue({
      answer: "命中资料的正文。",
      certainty: "rule_based",
      sources: [],
      human_review_notice: "请由负责人复核。",
    });

    render(<LearningCenterPage />);
    await screen.findByRole("heading", { name: "知识检索与问答" });
    fireEvent.change(screen.getByLabelText("搜索知识"), { target: { value: "如何处理" } });
    fireEvent.click(screen.getByRole("button", { name: "基于资料问答" }));

    expect(await screen.findByText("当前未配置或无法连接可用的 AI Gateway；以下是命中资料原文拼接，不是模型生成回答。")).toBeInTheDocument();
    expect(mocked.askKnowledge).toHaveBeenCalledWith("learner-session", "如何处理", { category: "", tag: "" });
    expect(screen.getByText("请由负责人复核。")).toBeInTheDocument();
  });

  it("identifies an answer generated by the configured AI Gateway", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.askKnowledge.mockResolvedValue({
      answer: "仅基于引用资料的回答。",
      certainty: "source_backed",
      sources: [],
      human_review_notice: "请由负责人复核。",
    });

    render(<LearningCenterPage />);
    await screen.findByRole("heading", { name: "知识检索与问答" });
    fireEvent.change(screen.getByLabelText("搜索知识"), { target: { value: "如何处理" } });
    fireEvent.click(screen.getByRole("button", { name: "基于资料问答" }));

    expect(await screen.findByText("AI Gateway 已基于下列已发布资料生成回答。")).toBeInTheDocument();
  });

  it("keeps the answer and explanation hidden until a single-choice submission completes", async () => {
    mocked.getPublicPreventionCard.mockRejectedValue(
      new ApiClientError(404, "not_found", "没有卡片"),
    );
    mocked.listLearningQuestions.mockResolvedValue([
      {
        id: "question-1",
        prompt: "哪项符合已发布资料？",
        question_type: "single_choice",
        difficulty: "basic",
        tags: ["基础"],
        options: [{ id: "a", text: "选项甲" }, { id: "b", text: "选项乙" }],
        definition: { options: [{ id: "a", text: "选项甲" }, { id: "b", text: "选项乙" }] },
        source_resource_id: "resource-1",
        version: 1,
      },
    ]);

    render(<LearningCenterPage />);
    expect(await screen.findByText("哪项符合已发布资料？")).toBeInTheDocument();
    expect(screen.queryByText("解析只会在提交后出现。")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "选项甲" }));
    await waitFor(() =>
      expect(mocked.submitLearningAnswer).toHaveBeenCalledWith(
        "learner-session",
        "question-1",
        "a",
      ),
    );
    expect(await screen.findByText("回答正确。", { exact: false })).toBeInTheDocument();
    expect(screen.getByText(/解析只会在提交后出现/)).toBeInTheDocument();
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
