import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { LearningCenterPage } from "./LearningCenterPage";

const mocked = vi.hoisted(() => ({
  getPublicPreventionCard: vi.fn(),
  listLearningQuestions: vi.fn(),
  listLearningResources: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({ token: "learner-session" }),
}));

vi.mock("../api/learning", () => ({
  getPublicPreventionCard: (...args: unknown[]) =>
    mocked.getPublicPreventionCard(...args),
  listLearningQuestions: (...args: unknown[]) =>
    mocked.listLearningQuestions(...args),
  listLearningResources: (...args: unknown[]) =>
    mocked.listLearningResources(...args),
  askKnowledge: vi.fn(),
  submitLearningAnswer: vi.fn(),
}));

describe("LearningCenterPage", () => {
  beforeEach(() => {
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
    expect(screen.getByText("可离线使用")).toBeInTheDocument();
    expect(screen.getByText("来源：指定负责人 · v2")).toBeInTheDocument();
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
});
