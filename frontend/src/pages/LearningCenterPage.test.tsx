import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { LearningCenterPage } from "./LearningCenterPage";

const mocked = vi.hoisted(() => ({
  getPublicPreventionCard: vi.fn(),
  askKnowledge: vi.fn(),
  listLearningQuestions: vi.fn(),
  listLearningResources: vi.fn(),
  token: "learner-session" as string | null,
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({ token: mocked.token }),
}));

vi.mock("../api/learning", () => ({
  getPublicPreventionCard: (...args: unknown[]) =>
    mocked.getPublicPreventionCard(...args),
  listLearningQuestions: (...args: unknown[]) =>
    mocked.listLearningQuestions(...args),
  listLearningResources: (...args: unknown[]) =>
    mocked.listLearningResources(...args),
  askKnowledge: (...args: unknown[]) => mocked.askKnowledge(...args),
  submitLearningAnswer: vi.fn(),
}));

describe("LearningCenterPage", () => {
  beforeEach(() => {
    mocked.token = "learner-session";
    mocked.listLearningResources.mockResolvedValue([]);
    mocked.listLearningQuestions.mockResolvedValue([]);
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
    expect(screen.getByText("离线缓存尚未就绪，请保持联网查看。")).toBeInTheDocument();
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
      screen.getByText("负责人尚未发布可离线使用的防走失知识卡。该卡发布并加载成功后，生产环境会保留最后一个已审核版本供离线查看。"),
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
    expect(screen.getByText("审核状态：已发布", { exact: false })).toBeInTheDocument();
    expect(screen.getByText("生效时间：", { exact: false })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("输入学习问题"), {
      target: { value: "如何复盘？" },
    });
    fireEvent.click(screen.getByRole("button", { name: "提交问题" }));

    expect(await screen.findByText("资料状态：已审核资料支持")).toBeInTheDocument();
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
      await screen.findByText("问答暂时不可用，已发布资料仍可查看。请稍后重试。"),
    ).toBeInTheDocument();
    expect(screen.getByText("已发布手册")).toBeInTheDocument();
  });
});
