import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { LearningGovernancePage } from "./LearningGovernancePage";

const mocked = vi.hoisted(() => ({
  listResources: vi.fn(),
  listQuestions: vi.fn(),
  transitionResource: vi.fn(),
  listCategories: vi.fn(),
  transitionCategory: vi.fn(),
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
  listManagedLearningResources: (...args: unknown[]) =>
    mocked.listResources(...args),
  listManagedLearningQuestions: (...args: unknown[]) =>
    mocked.listQuestions(...args),
  listManagedLearningCategories: (...args: unknown[]) =>
    mocked.listCategories(...args),
  transitionManagedLearningCategory: (...args: unknown[]) =>
    mocked.transitionCategory(...args),
  transitionManagedLearningResource: (...args: unknown[]) =>
    mocked.transitionResource(...args),
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
    mocked.listCategories.mockResolvedValue([]);
    mocked.transitionResource.mockReset();
    mocked.transitionCategory.mockReset();
    mocked.transitionCategory.mockResolvedValue({
      id: "category-1",
      name: "安全基础",
      status: "enabled",
    });
  });

  it("does not allow the submitter to confirm de-identification", async () => {
    render(<LearningGovernancePage />);

    const button = await screen.findByRole("button", { name: "确认脱敏" });
    expect(button).toBeDisabled();
    expect(
      screen.getByText("提交人不能确认本条内容的脱敏，请由另一名管理员处理。"),
    ).toBeInTheDocument();
  });

  it("does not offer abandoning an unmanaged record even when it has a predecessor id", async () => {
    const unmanaged = submittedResource("");
    mocked.listResources.mockResolvedValue([
      {
        ...unmanaged,
        previous_version_id: "resource-previous",
        lifecycle: { ...unmanaged.lifecycle, state: "unmanaged" },
      },
    ]);
    render(<LearningGovernancePage />);

    await screen.findByText(unmanaged.title);
    expect(
      screen.queryByRole("button", { name: "放弃更正" }),
    ).not.toBeInTheDocument();
  });

  it("limits a corrected question to its original source resource", async () => {
    const originalSource = {
      ...submittedResource("admin-2"),
      id: "resource-original",
      title: "原始来源",
      lifecycle: { ...submittedResource("admin-2").lifecycle, state: "published" },
    };
    const otherSource = {
      ...submittedResource("admin-2"),
      id: "resource-other",
      title: "其他来源",
      lifecycle: { ...submittedResource("admin-2").lifecycle, state: "published" },
    };
    mocked.listResources.mockResolvedValue([originalSource, otherSource]);
    mocked.listQuestions.mockResolvedValue([
      {
        id: "question-original",
        prompt: "原始题目",
        question_type: "single_choice",
        difficulty: "basic",
        tags: [],
        options: [],
        source_resource_id: "resource-original",
        version: 1,
        lifecycle: {
          ...submittedResource("admin-2").lifecycle,
          state: "published",
        },
      },
    ]);
    render(<LearningGovernancePage />);

    const correctionSelectors = await screen.findAllByLabelText("更正上一版本");
    fireEvent.change(correctionSelectors[1], {
      target: { value: "question-original" },
    });

    expect(screen.getByRole("option", { name: "原始来源" })).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "其他来源" }),
    ).not.toBeInTheDocument();
  });

  it("syncs the source resource when selecting a correction after choosing a source", async () => {
    const originalSource = {
      ...submittedResource("admin-2"),
      id: "resource-original",
      title: "Original source",
      lifecycle: { ...submittedResource("admin-2").lifecycle, state: "published" },
    };
    const otherSource = {
      ...submittedResource("admin-2"),
      id: "resource-other",
      title: "Other source",
      lifecycle: { ...submittedResource("admin-2").lifecycle, state: "published" },
    };
    mocked.listResources.mockResolvedValue([originalSource, otherSource]);
    mocked.listQuestions.mockResolvedValue([
      {
        id: "question-original",
        prompt: "Original question",
        question_type: "single_choice",
        difficulty: "basic",
        tags: [],
        options: [],
        source_resource_id: "resource-original",
        version: 1,
        lifecycle: {
          ...submittedResource("admin-2").lifecycle,
          state: "published",
        },
      },
    ]);
    render(<LearningGovernancePage />);

    const sourceSelectors = await screen.findAllByLabelText("来源资源");
    fireEvent.change(sourceSelectors[0], {
      target: { value: "resource-other" },
    });
    const correctionSelectors = await screen.findAllByLabelText("更正上一版本");
    fireEvent.change(correctionSelectors[1], {
      target: { value: "question-original" },
    });

    expect(sourceSelectors[0]).toHaveValue("resource-original");
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

  it("offers category lifecycle actions and sends the administrator reason", async () => {
    mocked.listCategories.mockResolvedValue([
      {
        id: "category-pending",
        name: "安全基础",
        status: "pending",
        submitted_by_user_id: "learner-1",
        reviewed_by_user_id: null,
        created_at: "2026-08-05T00:00:00.000Z",
        updated_at: "2026-08-05T00:00:00.000Z",
      },
      {
        id: "category-enabled",
        name: "沟通技巧",
        status: "enabled",
        submitted_by_user_id: "learner-2",
        reviewed_by_user_id: "admin-1",
        created_at: "2026-08-04T00:00:00.000Z",
        updated_at: "2026-08-04T00:00:00.000Z",
      },
    ]);
    render(<LearningGovernancePage />);

    expect(await screen.findByText("安全基础")).toBeInTheDocument();
    expect(screen.getAllByText("沟通技巧").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "启用" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "驳回" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "停用" })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("操作理由"), {
      target: { value: "分类符合新人课程结构" },
    });
    fireEvent.click(screen.getByRole("button", { name: "启用" }));

    await waitFor(() =>
      expect(mocked.transitionCategory).toHaveBeenCalledWith(
        "admin-session",
        "category-pending",
        "enable",
        "分类符合新人课程结构",
      ),
    );
  });
});
