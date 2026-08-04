import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { LearningGovernancePage } from "./LearningGovernancePage";

const mocked = vi.hoisted(() => ({
  listResources: vi.fn(),
  listQuestions: vi.fn(),
  transitionResource: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({
    token: "admin-session",
    user: {
      id: "admin-1",
      email: "admin@example.test",
      display_name: "管理员",
      account_type: "member",
      global_capabilities: ["admin"],
    },
  }),
}));

vi.mock("../api/learning", () => ({
  listManagedLearningResources: (...args: unknown[]) => mocked.listResources(...args),
  listManagedLearningQuestions: (...args: unknown[]) => mocked.listQuestions(...args),
  transitionManagedLearningResource: (...args: unknown[]) => mocked.transitionResource(...args),
  transitionManagedLearningQuestion: vi.fn(),
  createManagedLearningResource: vi.fn(),
  createManagedLearningQuestion: vi.fn(),
}));

function submittedResource(submittedBy = "admin-1") {
  return {
    id: "resource-1",
    title: "测试资源",
    summary: "摘要",
    content: "正文",
    resource_type: "manual",
    tags: [],
    source_name: "审核来源",
    source_url: null,
    version: 1,
    effective_at: "2026-08-04T00:00:00.000Z",
    lifecycle: {
      submitted_by_user_id: submittedBy,
      deidentified_by_user_id: null,
      reviewed_by_user_id: null,
      published_by_user_id: null,
      withdrawn_by_user_id: null,
      state: "submitted",
      permitted_use: "training",
      events: [],
    },
  };
}

describe("LearningGovernancePage", () => {
  beforeEach(() => {
    mocked.listResources.mockResolvedValue([submittedResource()]);
    mocked.listQuestions.mockResolvedValue([]);
    mocked.transitionResource.mockReset();
  });

  it("does not allow the submitter to confirm de-identification", async () => {
    render(<LearningGovernancePage />);

    const button = await screen.findByRole("button", { name: "确认脱敏" });
    expect(button).toBeDisabled();
    expect(
      screen.getByText("提交人不能确认本条内容的脱敏，请由另一名管理员处理。"),
    ).toBeInTheDocument();
  });

  it("keeps a governance conflict in the current page with its specific reason", async () => {
    mocked.listResources.mockResolvedValue([submittedResource("admin-2")]);
    mocked.transitionResource.mockRejectedValue(
      new ApiClientError(
        409,
        "conflict",
        "当前数据状态已变化，请刷新后再试。",
        "脱敏和审核必须由非提交人按顺序完成",
      ),
    );
    render(<LearningGovernancePage />);

    fireEvent.change(await screen.findByLabelText("操作理由"), {
      target: { value: "完成脱敏检查" },
    });
    fireEvent.click(screen.getByRole("button", { name: "确认脱敏" }));

    await waitFor(() =>
      expect(
        screen.getByText("脱敏和审核必须由非提交人按顺序完成"),
      ).toBeInTheDocument(),
    );
    expect(
      screen.getByRole("heading", { name: "学习内容治理" }),
    ).toBeInTheDocument();
  });
});
