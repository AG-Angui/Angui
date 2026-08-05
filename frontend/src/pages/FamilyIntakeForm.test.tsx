import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  IntakeDraft,
  IntakeSession,
  SubmitIntakeAnswerResponse,
} from "../api/intake";
import { ApiClientError } from "../api/client";
import { FamilyIntakeForm } from "./FamilyIntakeForm";

const mocked = vi.hoisted(() => ({
  confirmIntakeSession: vi.fn(),
  createIntakeSession: vi.fn(),
  getIntakeAiInitialReview: vi.fn(),
  getIntakeAiFollowUp: vi.fn(),
  getIntakeDraft: vi.fn(),
  listIntakeAnswerRevisions: vi.fn(),
  listIntakeDraftVersions: vi.fn(),
  acknowledgeIntakeAiInitialReview: vi.fn(),
  startIntakeAiInitialReview: vi.fn(),
  submitIntakeAnswer: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({ token: "family-session", user: { id: "family-1" } }),
}));

vi.mock("../api/intake", () => ({
  confirmIntakeSession: (...args: unknown[]) =>
    mocked.confirmIntakeSession(...args),
  createIntakeSession: (...args: unknown[]) =>
    mocked.createIntakeSession(...args),
  getIntakeAiInitialReview: (...args: unknown[]) =>
    mocked.getIntakeAiInitialReview(...args),
  getIntakeAiFollowUp: (...args: unknown[]) =>
    mocked.getIntakeAiFollowUp(...args),
  getIntakeDraft: (...args: unknown[]) => mocked.getIntakeDraft(...args),
  listIntakeAnswerRevisions: (...args: unknown[]) =>
    mocked.listIntakeAnswerRevisions(...args),
  listIntakeDraftVersions: (...args: unknown[]) =>
    mocked.listIntakeDraftVersions(...args),
  acknowledgeIntakeAiInitialReview: (...args: unknown[]) =>
    mocked.acknowledgeIntakeAiInitialReview(...args),
  startIntakeAiInitialReview: (...args: unknown[]) =>
    mocked.startIntakeAiInitialReview(...args),
  submitIntakeAnswer: (...args: unknown[]) =>
    mocked.submitIntakeAnswer(...args),
}));

const collectingSession: IntakeSession = {
  id: "intake-1",
  status: "collecting",
  missing_fields: ["last_seen"],
  phase: "phase_one",
  completed_phase_one_fields: ["basic_information"],
  missing_phase_one_fields: ["last_seen"],
  phase_transition_ready: false,
  next_question: {
    field: "last_seen",
    prompt: "请描述最后出现的地点和时间",
    required: true,
  },
  guidance_mode: "rule_based",
  ai_initial_review_status: "not_started",
  privacy_notice: "仅用于本次问询。",
};

const readySession: IntakeSession = {
  ...collectingSession,
  status: "ready_for_confirmation",
  completed_phase_one_fields: [
    "basic_information",
    "health_status",
    "behavior_habits",
    "last_seen",
  ],
  missing_phase_one_fields: [],
  phase_transition_ready: true,
  next_question: null,
};

const basicInformationSession: IntakeSession = {
  ...collectingSession,
  completed_phase_one_fields: [],
  missing_phase_one_fields: ["basic_information", "last_seen"],
  next_question: {
    field: "basic_information",
    prompt: "请填写可供家属核对的基础信息。",
    required: true,
  },
};

const afterBasicInformationSession: IntakeSession = {
  ...basicInformationSession,
  completed_phase_one_fields: ["basic_information"],
  missing_phase_one_fields: ["last_seen"],
  next_question: {
    field: "health_status",
    prompt: "请填写健康情况。",
    required: false,
  },
};

const phaseTwoSession: IntakeSession = {
  ...collectingSession,
  phase: "phase_two",
  completed_phase_one_fields: [
    "basic_information",
    "health_status",
    "behavior_habits",
    "last_seen",
  ],
  missing_phase_one_fields: [],
  phase_transition_ready: true,
  missing_fields: ["frequent_locations"],
  next_question: {
    field: "frequent_locations",
    prompt: "请补充常去地点。",
    required: false,
  },
};

const profileDraft: IntakeDraft = {
  id: "profile-draft-test-1",
  status: "draft",
  source_scope: "family_provided intake answers from this session only",
  generated_at: "2026-07-25T08:00:00Z",
  provider_model: null,
  template_version: "test",
  degradation_status: "rule_based_fallback",
  version: 1,
  requires_human_confirmation: true,
  profile: {
    physical_description: "佩戴眼镜，穿蓝色外套。",
    clothing_description: null,
    health_notes: "行动较慢，需要留意。",
    mobility_notes: "行动较慢，需要留意。",
    transportation_ability: null,
    frequent_locations: null,
    last_seen_information: "模拟社区北门",
    behavior_habits: null,
    suspicious_motive: null,
  },
  field_metadata: [
    {
      field: "physical_description",
      source_field: "basic_information",
      source: "family_provided",
      status: "draft",
      generated_at: "2026-07-25T08:00:00Z",
    },
    {
      field: "health_notes",
      source_field: "health_status",
      source: "family_provided",
      status: "draft",
      generated_at: "2026-07-25T08:01:00Z",
    },
    {
      field: "last_seen_information",
      source_field: "last_seen",
      source: "family_provided",
      status: "draft",
      generated_at: "2026-07-25T08:02:00Z",
    },
  ],
  missing_fields: [],
  assessments: [],
  confirmation_blocked_reasons: [],
  direction_hypotheses: [],
};

function answerResponse(session: IntakeSession): SubmitIntakeAnswerResponse {
  const { id: session_id, ...sessionUpdate } = session;
  return {
    ...sessionUpdate,
    session_id,
    raw_answer: "模拟社区北门",
    candidate_fields: [
      {
        field: "last_seen",
        value: "模拟社区北门",
        source: "family_provided",
        status: "draft",
        generated_at: "2026-07-25T08:02:00Z",
        model: null,
        template_version: null,
        source_text: "模拟社区北门",
        confidence: null,
      },
    ],
    assessments: [],
  };
}

describe("FamilyIntakeForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.getIntakeAiFollowUp.mockResolvedValue({
      question: null,
      degradation_status: "rule_based_fallback",
      generated_at: "2026-07-25T08:00:00Z",
    });
    mocked.listIntakeAnswerRevisions.mockResolvedValue([]);
    mocked.listIntakeDraftVersions.mockResolvedValue({ items: [] });
    window.sessionStorage.clear();
  });

  it("restores the current-tab draft and does not create a second intake session", async () => {
    window.sessionStorage.setItem(
      "angui:intake-tab-draft:family-1",
      JSON.stringify({
        session: collectingSession,
        answer: "尚未提交的地点描述",
      }),
    );

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: "最后出现情况" }),
    ).toBeInTheDocument();
    expect(screen.getByDisplayValue("尚未提交的地点描述")).toBeInTheDocument();
    expect(mocked.createIntakeSession).not.toHaveBeenCalled();
  });

  it("discards a malformed stored session before rendering the intake flow", async () => {
    const malformedSession = {
      ...collectingSession,
      missing_phase_one_fields: undefined,
    };
    window.sessionStorage.setItem(
      "angui:intake-tab-draft:family-1",
      JSON.stringify({ session: malformedSession, answer: "stale answer" }),
    );

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      await screen.findByRole("button", { name: "开始问询" }),
    ).toBeInTheDocument();
    expect(
      window.sessionStorage.getItem("angui:intake-tab-draft:family-1"),
    ).toBeNull();
  });

  it.each([
    ["an invalid status", { ...collectingSession, status: "stale" }],
    ["an invalid phase", { ...collectingSession, phase: "unknown_phase" }],
    [
      "a malformed next question",
      {
        ...collectingSession,
        next_question: { field: "last_seen", prompt: "Missing required flag" },
      },
    ],
  ])(
    "discards cached sessions with %s",
    async (_description, malformedSession) => {
      window.sessionStorage.setItem(
        "angui:intake-tab-draft:family-1",
        JSON.stringify({ session: malformedSession, answer: "" }),
      );

      render(
        <FamilyIntakeForm
          onCancel={vi.fn()}
          onConfirmed={vi.fn().mockResolvedValue(undefined)}
        />,
      );

      expect(
        await screen.findByRole("button", { name: "开始问询" }),
      ).toBeInTheDocument();
      expect(
        window.sessionStorage.getItem("angui:intake-tab-draft:family-1"),
      ).toBeNull();
    },
  );

  it.each([403, 404])(
    "clears a %i unavailable ready-for-confirmation session",
    async (status) => {
      window.sessionStorage.setItem(
        "angui:intake-tab-draft:family-1",
        JSON.stringify({ session: readySession, answer: "" }),
      );
      const message = `Draft is no longer available (${status})`;
      mocked.getIntakeDraft.mockRejectedValue(
        new ApiClientError(
          status,
          status === 403 ? "forbidden" : "not_found",
          message,
        ),
      );

      render(
        <FamilyIntakeForm
          onCancel={vi.fn()}
          onConfirmed={vi.fn().mockResolvedValue(undefined)}
        />,
      );

      expect(
        await screen.findByRole("button", { name: "开始问询" }),
      ).toBeInTheDocument();
      expect(
        window.sessionStorage.getItem("angui:intake-tab-draft:family-1"),
      ).toBeNull();
      expect(screen.getByText(message)).toBeInTheDocument();
    },
  );

  it("submits basic information from labelled fields instead of a single free-text box", async () => {
    mocked.createIntakeSession.mockResolvedValue(basicInformationSession);
    mocked.submitIntakeAnswer.mockResolvedValue(
      answerResponse(afterBasicInformationSession),
    );

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "开始问询" }));
    await screen.findByRole("heading", { name: "基本信息" });
    fireEvent.change(screen.getByRole("textbox", { name: "姓名或称呼" }), {
      target: { value: "王女士" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "性别" }), {
      target: { value: "女" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "年龄" }), {
      target: { value: "0" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "身高（厘米）" }), {
      target: { value: "158" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "便于识别的外观特征" }),
      { target: { value: "短发，戴眼镜" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "保存并继续" }));

    await waitFor(() =>
      expect(mocked.submitIntakeAnswer).toHaveBeenCalledWith(
        "family-session",
        "intake-1",
        {
          field: "basic_information",
          answer:
            "姓名或称呼：王女士\n性别：女\n年龄：0 岁\n身高：158 厘米\n外观特征：短发，戴眼镜",
          replace: false,
        },
      ),
    );
  });

  it("omits a blank age from the structured basic-information answer", async () => {
    mocked.createIntakeSession.mockResolvedValue(basicInformationSession);
    mocked.submitIntakeAnswer.mockResolvedValue(
      answerResponse(afterBasicInformationSession),
    );

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "开始问询" }));
    await screen.findByRole("heading", { name: "基本信息" });
    fireEvent.change(screen.getByRole("textbox", { name: "姓名或称呼" }), {
      target: { value: "王女士" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "年龄" }), {
      target: { value: "   " },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存并继续" }));

    await waitFor(() =>
      expect(mocked.submitIntakeAnswer).toHaveBeenCalledWith(
        "family-session",
        "intake-1",
        {
          field: "basic_information",
          answer: "姓名或称呼：王女士",
          replace: false,
        },
      ),
    );
  });

  it("returns to basic information from phase two without losing the clue draft", async () => {
    window.sessionStorage.setItem(
      "angui:intake-tab-draft:family-1",
      JSON.stringify({
        session: phaseTwoSession,
        answer: "北门附近有目击信息。",
        basicInformation: {
          name: "王女士",
          gender: "女",
          age: "72",
          height: "158",
          appearance: "短发，戴眼镜",
        },
      }),
    );
    mocked.submitIntakeAnswer.mockResolvedValue(
      answerResponse(phaseTwoSession),
    );

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: "常去地点" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "上一步" }));

    expect(
      await screen.findByRole("heading", { name: "基本情况（编辑）" }),
    ).toBeInTheDocument();
    expect(screen.getByDisplayValue("王女士")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: "姓名或称呼" }), {
      target: { value: "王女士（已更正）" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "保存更正并返回补充线索" }),
    );

    await waitFor(() =>
      expect(mocked.submitIntakeAnswer).toHaveBeenCalledWith(
        "family-session",
        "intake-1",
        expect.objectContaining({
          field: "basic_information",
          answer: expect.stringContaining("王女士（已更正）"),
          replace: true,
        }),
      ),
    );
    expect(
      await screen.findByRole("heading", { name: "常去地点" }),
    ).toBeInTheDocument();
    expect(
      screen.getByDisplayValue("北门附近有目击信息。"),
    ).toBeInTheDocument();
  });

  it("uses Chinese default prompts when an existing session still returns the old seed copy", async () => {
    window.sessionStorage.setItem(
      "angui:intake-tab-draft:family-1",
      JSON.stringify({
        session: {
          ...phaseTwoSession,
          next_question: {
            field: "follow_up_clues",
            prompt:
              "Is there later information or a lead that still needs human verification?",
            required: false,
          },
        },
        answer: "",
        basicInformation: {
          name: "",
          gender: "",
          age: "",
          height: "",
          appearance: "",
        },
      }),
    );

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    expect(
      await screen.findByText("是否有之后获得、但仍需要人工核实的信息或线索？"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(
        "Is there later information or a lead that still needs human verification?",
      ),
    ).not.toBeInTheDocument();
  });

  it("shows field-level provenance and sends a replacement when the family corrects a draft answer", async () => {
    mocked.createIntakeSession.mockResolvedValue(collectingSession);
    mocked.submitIntakeAnswer.mockResolvedValue(answerResponse(readySession));
    mocked.getIntakeDraft.mockResolvedValue(profileDraft);

    render(
      <FamilyIntakeForm
        onCancel={vi.fn()}
        onConfirmed={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "开始问询" }));
    await screen.findByRole("heading", { name: "最后出现情况" });
    fireEvent.change(
      screen.getByRole("textbox", { name: /请描述最后出现的地点和时间/ }),
      { target: { value: "模拟社区北门" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "保存并继续" }));

    expect(await screen.findByText("问询整理出的画像草稿")).toBeInTheDocument();
    expect(mocked.getIntakeDraft).toHaveBeenCalledWith(
      "family-session",
      "intake-1",
    );
    expect(screen.getByText("来源：家属提供 · 健康情况")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "修改健康情况" }));
    const editInput = screen.getByRole("textbox", { name: "修订后的家属回答" });
    fireEvent.change(editInput, {
      target: { value: "家属已核对：行动不受限。" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存修订并刷新草稿" }));

    await waitFor(() =>
      expect(mocked.submitIntakeAnswer).toHaveBeenLastCalledWith(
        "family-session",
        "intake-1",
        {
          field: "health_status",
          answer: "家属已核对：行动不受限。",
          replace: true,
        },
      ),
    );
  });

  it("requires an explicit second confirmation before it creates a case", async () => {
    window.sessionStorage.setItem(
      "angui:intake-tab-draft:family-1",
      JSON.stringify({ session: readySession, answer: "" }),
    );
    mocked.getIntakeDraft.mockResolvedValue(profileDraft);
    mocked.startIntakeAiInitialReview.mockResolvedValue({
      session_id: "intake-1",
      status: "awaiting_family_review",
      degradation_status: "available",
      issues: [
        {
          id: "issue-1",
          field: "last_seen",
          severity: "needs_confirmation",
          evidence_summary:
            "The reported time and place need family confirmation.",
          clarification_question:
            "Please confirm the last-seen information is accurate.",
          source_fields: ["last_seen"],
        },
      ],
      blocking_assessments: [],
      generated_at: "2026-07-25T08:10:00Z",
      requires_family_acknowledgement: true,
      ready_for_second_confirmation: false,
    });
    mocked.acknowledgeIntakeAiInitialReview.mockResolvedValue({
      session_id: "intake-1",
      status: "ready_for_second_confirmation",
      degradation_status: "available",
      issues: [
        {
          id: "issue-1",
          field: "last_seen",
          severity: "needs_confirmation",
          evidence_summary:
            "The reported time and place need family confirmation.",
          clarification_question:
            "Please confirm the last-seen information is accurate.",
          source_fields: ["last_seen"],
        },
      ],
      blocking_assessments: [],
      generated_at: "2026-07-25T08:10:00Z",
      requires_family_acknowledgement: false,
      ready_for_second_confirmation: true,
    });
    mocked.confirmIntakeSession.mockResolvedValue({
      case_id: "case-1",
      case_code: "AG-0001",
      status: "active",
      confirmation_status: "human_confirmed",
      confirmed_at: "2026-07-25T08:10:00Z",
    });
    const onConfirmed = vi.fn().mockResolvedValue(undefined);

    render(<FamilyIntakeForm onCancel={vi.fn()} onConfirmed={onConfirmed} />);

    await screen.findByText("确认后写入案件的资料");
    await waitFor(() =>
      expect(screen.getByRole("textbox", { name: "最后出现地点" })).toHaveValue(
        "模拟社区北门",
      ),
    );
    fireEvent.change(screen.getByRole("textbox", { name: "姓名或称呼" }), {
      target: { value: "模拟老人" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "首次确认并进行 AI 初步审核" }),
    );

    expect(mocked.confirmIntakeSession).not.toHaveBeenCalled();
    await screen.findByText("AI 初步审核结果");
    expect(mocked.startIntakeAiInitialReview).toHaveBeenCalledTimes(1);
    expect(mocked.confirmIntakeSession).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(screen.getByRole("button", { name: "确认初审标注" }));
    await waitFor(() =>
      expect(mocked.acknowledgeIntakeAiInitialReview).toHaveBeenCalledWith(
        "family-session",
        "intake-1",
        ["issue-1"],
      ),
    );
    expect(mocked.confirmIntakeSession).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: "二次确认并提交指挥端" }),
    );
    await waitFor(() =>
      expect(mocked.confirmIntakeSession).toHaveBeenCalledTimes(1),
    );
    expect(mocked.confirmIntakeSession).toHaveBeenCalledWith(
      "family-session",
      "intake-1",
      expect.objectContaining({ age: null }),
    );
    expect(onConfirmed).toHaveBeenCalledWith("case-1", "AG-0001");
  });
});
