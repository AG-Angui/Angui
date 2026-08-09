import { Button, Chip, Input, Spinner, TextArea } from "@heroui/react";
import {
  AlertTriangle,
  ArrowLeft,
  CheckCircle2,
  CircleHelp,
  FilePenLine,
  ShieldCheck,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ApiClientError, type SseEvent } from "../api/client";
import { getAiExecution } from "../api/aiExecutions";
import {
  acknowledgeIntakeAiInitialReview,
  confirmIntakeSession,
  createIntakeSession,
  getIntakeAiInitialReview,
  getIntakeAiFollowUp,
  getIntakeDraft,
  generateIntakeDraft,
  diffIntakeDraftVersions,
  listIntakeDraftVersions,
  reviewIntakeDraft,
  restoreIntakeDraft,
  listIntakeAnswerRevisions,
  restoreIntakeAnswerRevision,
  listIntakePhotos,
  startIntakeAiInitialReview,
  submitIntakeAnswer,
  uploadIntakePhoto,
} from "../api/intake";
import type {
  ConfirmedIntakeProfile,
  IntakeAiFollowUpResponse,
  IntakeAiInitialReviewResponse,
  IntakeAssessment,
  IntakeAnswerRevision,
  IntakeDraft,
  IntakeDraftProfile,
  IntakeProfileDraftFieldMetadata,
  IntakePhoto,
  IntakeSession,
  SubmitIntakeAnswerResponse,
} from "../api/intake";
import { useAuth } from "../auth/useAuth";
import {
  AiReviewProgress,
  type AiReviewStage,
} from "../components/AiReviewProgress";

const questionLabels: Record<string, string> = {
  basic_information: "基本信息",
  police_report_status: "是否报警",
  family_phone: "家属电话",
  health_status: "健康情况",
  behavior_habits: "行为习惯",
  last_seen: "最后出现情况",
  frequent_locations: "常去地点",
  suspicious_motive: "可疑动机",
  belongings: "随身物品与衣着",
  transport_ability: "出行能力",
  follow_up_clues: "后续线索",
};

const questionReasons: Record<string, string> = {
  basic_information: "用于区分被寻找的人，后续仍需要家属逐项核对。",
  health_status: "帮助现场人员理解可能需要的照护与沟通方式。",
  behavior_habits: "帮助判断可能的行动习惯，不会自动成为正式事实。",
  last_seen: "用于建立初始的时间和地点线索。",
  frequent_locations: "可作为待核实的寻找方向，不会直接发布。",
  suspicious_motive: "仅用于记录家属的待核实判断。",
  belongings: "便于后续人工比对衣着和随身物品。",
  transport_ability: "帮助评估可能的行动范围。",
  follow_up_clues: "记录家属尚待核实的补充信息。",
};

const defaultQuestionPrompts: Record<string, string> = {
  basic_information: "请填写可供家属核对的基本信息。",
  health_status: "请补充健康、认知、行动能力或用药方面需要记录的情况。",
  behavior_habits: "请描述有助于后续核实线索的日常习惯、偏好或行为特点。",
  last_seen:
    "请说明最后出现的时间和地点；如有不确定的交通方式或同行人，也请标明。",
  frequent_locations: "请补充常去地点，并避免填写与寻找无关的私人住址。",
  suspicious_motive:
    "是否有需要人工谨慎核实的可能原因、计划或担忧？不确定时可标记为未知。",
  belongings: "请描述当时携带的衣着、包、手机、证件或其他随身物品。",
  transport_ability:
    "请说明可能的独立出行方式，包括步行、车辆、公共交通及同行人情况。",
  follow_up_clues: "是否有之后获得、但仍需要人工核实的信息或线索？",
};

const legacyDefaultQuestionPrompts: Record<string, string> = {
  basic_information:
    "Please describe the person using information your family can verify.",
  health_status:
    "What health, cognitive, mobility, or medication concerns should be recorded as unconfirmed draft information?",
  behavior_habits:
    "What routines, preferences, or behaviors may help verify future leads?",
  last_seen:
    "When and where was the person last seen? Include uncertainty in time, place, transport, or companions.",
  frequent_locations:
    "Which places do they commonly visit? Please avoid unrelated private addresses.",
  suspicious_motive:
    "Are there any possible reasons, plans, or concerns that need careful human follow-up? Mark unknown when unsure.",
  belongings:
    "What clothing, bags, phone, identification, or other belongings were they carrying?",
  transport_ability:
    "How might they travel independently? Include walking, vehicle, public transport, and companion uncertainty.",
  follow_up_clues:
    "Is there later information or a lead that still needs human verification?",
};

const blankProfile: ConfirmedIntakeProfile = {
  display_name: "",
  age: null,
  gender: null,
  physical_description: null,
  clothing_description: null,
  health_notes: null,
  last_seen_at: null,
  last_seen_location: "",
};

type StoredIntakeSession = Pick<
  IntakeSession,
  | "id"
  | "question_set_version"
  | "status"
  | "missing_fields"
  | "phase"
  | "completed_phase_one_fields"
  | "missing_phase_one_fields"
  | "phase_transition_ready"
  | "next_question"
  | "guidance_mode"
  | "ai_initial_review_status"
  | "privacy_notice"
>;

interface StoredIntakeState {
  session: StoredIntakeSession;
  answer: string;
  basicInformation: BasicInformationDraft;
  aiExecution?: { id: string; workflow: string };
}

interface BasicInformationDraft {
  name: string;
  gender: string;
  age: string;
  height: string;
  appearance: string;
}

const blankBasicInformation: BasicInformationDraft = {
  name: "",
  gender: "",
  age: "",
  height: "",
  appearance: "",
};

function isPhaseOneIntakeField(field: string, questionSetVersion: number) {
  const phaseOneFields =
    questionSetVersion >= 3
      ? [
          "basic_information",
          "last_seen",
          "suspicious_motive",
          "police_report_status",
          "family_phone",
        ]
      : [
          "basic_information",
          "health_status",
          "behavior_habits",
          "last_seen",
        ];
  return phaseOneFields.includes(field);
}

export function FamilyIntakeForm({
  onCancel,
  onConfirmed,
}: {
  onCancel: () => void;
  onConfirmed: (caseId: string, caseCode: string) => Promise<void>;
}) {
  const { token, user } = useAuth();
  const storageKey = `angui:intake-tab-draft:${user?.id ?? "anonymous"}`;
  const [session, setSession] = useState<IntakeSession | null>(null);
  const [draft, setDraft] = useState<IntakeDraft | null>(null);
  const [answer, setAnswer] = useState("");
  const [basicInformation, setBasicInformation] =
    useState<BasicInformationDraft>(blankBasicInformation);
  const [profile, setProfile] = useState<ConfirmedIntakeProfile>(blankProfile);
  const [photos, setPhotos] = useState<IntakePhoto[]>([]);
  const [assessments, setAssessments] = useState<IntakeAssessment[]>([]);
  const [editSource, setEditSource] =
    useState<IntakeProfileDraftFieldMetadata | null>(null);
  const [editAnswer, setEditAnswer] = useState("");
  const [confirmReviewOpen, setConfirmReviewOpen] = useState(false);
  const [initialReview, setInitialReview] =
    useState<IntakeAiInitialReviewResponse | null>(null);
  const [aiReviewStage, setAiReviewStage] = useState<AiReviewStage | null>(
    null,
  );
  const [activeAiExecution, setActiveAiExecution] = useState<{
    id: string;
    workflow: string;
  } | null>(null);
  const [confirmedInitialReviewIssues, setConfirmedInitialReviewIssues] =
    useState<string[]>([]);
  const [answerRevisions, setAnswerRevisions] = useState<
    IntakeAnswerRevision[]
  >([]);
  const [profileVersions, setProfileVersions] = useState<IntakeDraft[]>([]);
  const [comparison, setComparison] = useState<{
    from: string;
    to: string;
    fields: string[];
  } | null>(null);
  const [isReviewingBasicInformation, setIsReviewingBasicInformation] =
    useState(false);
  const [isFetchingAiFollowUp, setIsFetchingAiFollowUp] = useState(false);
  const [busyAction, setBusyAction] = useState<
    | "begin"
    | "answer"
    | "replace"
    | "generate"
    | "initial_review"
    | "acknowledge_initial_review"
    | "confirm"
    | "photo"
    | null
  >(null);
  const [error, setError] = useState("");
  const [hasHydrated, setHasHydrated] = useState(false);
  const confirmDialogRef = useRef<HTMLDivElement>(null);
  const basicInformationRef = useRef(basicInformation);
  const activeAiExecutionRef = useRef(activeAiExecution);

  useEffect(() => {
    basicInformationRef.current = basicInformation;
  }, [basicInformation]);

  useEffect(() => {
    activeAiExecutionRef.current = activeAiExecution;
  }, [activeAiExecution]);

  const updateAiReviewStage = useCallback(({ event, payload }: SseEvent) => {
    if (!payload || typeof payload !== "object") return;
    if (
      event === "started" &&
      "execution_id" in payload &&
      typeof payload.execution_id === "string" &&
      "workflow" in payload &&
      typeof payload.workflow === "string"
    ) {
      const execution = {
        id: payload.execution_id,
        workflow: payload.workflow,
      };
      activeAiExecutionRef.current = execution;
      setActiveAiExecution(execution);
      setAiReviewStage("queued");
    }
    if (event !== "progress") return;
    const stage = "stage" in payload ? payload.stage : null;
    if (
      stage === "queued" ||
      stage === "preparing" ||
      stage === "generating" ||
      stage === "validating" ||
      stage === "fallback" ||
      stage === "ready_for_review" ||
      stage === "failed"
    ) {
      setAiReviewStage(stage);
    }
  }, []);

  const isBusy = busyAction !== null;
  const displayedAssessments = draft?.assessments ?? assessments;
  const sourceOptions = useMemo(() => uniqueSourceOptions(draft), [draft]);

  useEffect(() => {
    const sessionId = session?.id;
    if (!token || !sessionId) return;
    void listIntakePhotos(token, sessionId)
      .then(setPhotos)
      .catch(() => setPhotos([]));
  }, [session, token]);

  useEffect(() => {
    if (confirmReviewOpen) confirmDialogRef.current?.focus();
  }, [confirmReviewOpen]);

  const loadDraft = useCallback(
    async (
      sessionId: string,
      initializeProfile: boolean,
      basicInformationForProfile: BasicInformationDraft = blankBasicInformation,
    ) => {
      if (!token) return null;
      try {
        const nextDraft = await getIntakeDraft(token, sessionId);
        setDraft(nextDraft);
        setAssessments(nextDraft.assessments);
        void Promise.resolve()
          .then(() => listIntakeAnswerRevisions(token, sessionId))
          .then(setAnswerRevisions)
          .catch(() => setAnswerRevisions([]));
        void listIntakeDraftVersions(token, sessionId)
          .then((value) => setProfileVersions(value.items))
          .catch(() => setProfileVersions([]));
        if (initializeProfile)
          setProfile(profileFromDraft(nextDraft, basicInformationForProfile));
        return nextDraft;
      } catch (cause) {
        setError(messageFrom(cause));
        if (
          cause instanceof ApiClientError &&
          (cause.status === 403 || cause.status === 404)
        ) {
          clearStoredState(storageKey);
          setSession(null);
          setDraft(null);
          setAssessments([]);
          setAnswer("");
          setBasicInformation(blankBasicInformation);
          setProfile(blankProfile);
          setEditSource(null);
          setEditAnswer("");
          setConfirmReviewOpen(false);
        }
        return null;
      }
    },
    [storageKey, token],
  );

  useEffect(() => {
    setHasHydrated(false);
    if (!token || typeof window === "undefined") {
      setHasHydrated(true);
      return;
    }

    const stored = readStoredState(storageKey);
    if (stored) {
      setSession(stored.session);
      setAnswer(stored.answer);
      setBasicInformation(stored.basicInformation);
      if (stored.aiExecution) setActiveAiExecution(stored.aiExecution);
      if (
        [
          "ready_for_confirmation",
          "awaiting_family_review",
          "ready_for_second_confirmation",
        ].includes(stored.session.status)
      ) {
        void loadDraft(stored.session.id, true, stored.basicInformation);
      }
      if (
        ["awaiting_family_review", "ready_for_second_confirmation"].includes(
          stored.session.status,
        )
      ) {
        void getIntakeAiInitialReview(token, stored.session.id)
          .then((review) => {
            setInitialReview(review);
            setConfirmedInitialReviewIssues(
              review.ready_for_second_confirmation
                ? review.issues.map((item) => item.id)
                : [],
            );
          })
          .catch((cause) => setError(messageFrom(cause)));
      }
    }
    setHasHydrated(true);
  }, [loadDraft, storageKey, token]);

  useEffect(() => {
    if (!hasHydrated || typeof window === "undefined") return;
    if (!session) {
      window.sessionStorage.removeItem(storageKey);
      return;
    }
    const stored: StoredIntakeState = {
      session: toStoredSession(session),
      answer,
      basicInformation,
      aiExecution: activeAiExecution ?? undefined,
    };
    window.sessionStorage.setItem(storageKey, JSON.stringify(stored));
  }, [
    activeAiExecution,
    answer,
    basicInformation,
    hasHydrated,
    session,
    storageKey,
  ]);

  useEffect(() => {
    if (!token || !session || !activeAiExecution) return;
    const sessionToken = token;
    const executionId = activeAiExecution.id;
    const workflow = activeAiExecution.workflow;
    const sessionId = session.id;
    let cancelled = false;
    let retry: number | undefined;
    let attempts = 0;
    let networkFailures = 0;
    const maxAttempts = 60;
    const maxNetworkFailures = 3;

    async function recover() {
      attempts += 1;
      if (attempts > maxAttempts) {
        if (!cancelled) {
          setAiReviewStage("failed");
          setError("AI 审核恢复超时，请重试或按现有人工流程继续。");
          setBusyAction(null);
        }
        return;
      }
      try {
        const execution = await getAiExecution(sessionToken, executionId);
        networkFailures = 0;
        if (cancelled) return;
        if (execution.status === "running") {
          setAiReviewStage(execution.stage);
          retry = window.setTimeout(recover, 2_000);
          return;
        }
        if (execution.status === "failed") {
          setAiReviewStage("failed");
          setError("AI 审核未能完成，请重试或按现有人工流程继续。");
          setBusyAction(null);
          return;
        }
        if (workflow === "intake_profile_draft") {
          await loadDraft(sessionId, true, basicInformationRef.current);
        } else if (workflow === "intake_initial_review") {
          const review = await getIntakeAiInitialReview(sessionToken, sessionId);
          if (!cancelled) {
            setInitialReview(review);
            setConfirmedInitialReviewIssues([]);
            setSession((current) =>
              current ? { ...current, status: review.status } : current,
            );
          }
        }
        if (!cancelled) {
          setAiReviewStage(null);
          setActiveAiExecution(null);
          setBusyAction(null);
        }
      } catch (cause) {
        if (!cancelled) {
          networkFailures += 1;
          if (networkFailures <= maxNetworkFailures && attempts < maxAttempts) {
            retry = window.setTimeout(recover, 2_000);
          } else {
            setAiReviewStage("failed");
            setError(messageFrom(cause));
            setBusyAction(null);
          }
        }
      }
    }

    void recover();
    return () => {
      cancelled = true;
      if (retry !== undefined) window.clearTimeout(retry);
    };
  }, [activeAiExecution, loadDraft, session, token]);

  useEffect(() => {
    if (!answer.trim() || typeof window === "undefined") return;
    const warnBeforeLeave = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warnBeforeLeave);
    return () => window.removeEventListener("beforeunload", warnBeforeLeave);
  }, [answer]);

  async function begin() {
    if (!token) return;
    setBusyAction("begin");
    setError("");
    try {
      const nextSession = await createIntakeSession(token);
      setSession(nextSession);
      setDraft(null);
      setInitialReview(null);
      setAiReviewStage(null);
      setActiveAiExecution(null);
      setConfirmedInitialReviewIssues([]);
      setAssessments([]);
      setAnswer("");
      setBasicInformation(blankBasicInformation);
      setPhotos([]);
      setIsReviewingBasicInformation(false);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function sendAnswer(
    field: string,
    value: string,
    replace = false,
  ): Promise<boolean> {
    if (!token || !session || !value.trim()) {
      setError("请填写答案，或选择“标记为未知”。");
      return false;
    }
    setBusyAction(replace ? "replace" : "answer");
    setError("");
    try {
      const response = await submitIntakeAnswer(token, session.id, {
        field,
        answer: value.trim(),
        replace,
      });
      const next = sessionFromAnswerResponse(response);
      let guidedSession = next;
      if (!isPhaseOneIntakeField(field, session.question_set_version)) {
        setIsFetchingAiFollowUp(true);
        let guidance: IntakeAiFollowUpResponse;
        try {
          guidance = await getIntakeAiFollowUp(token, next.id);
        } finally {
          setIsFetchingAiFollowUp(false);
        }
        guidedSession = guidance.question
          ? {
              ...next,
              next_question: {
                field: guidance.question.field,
                prompt: guidance.question.prompt,
                required: false,
              },
              guidance_mode:
                guidance.degradation_status === "available"
                  ? ("ai_assisted" as const)
                  : ("rule_based" as const),
            }
          : next;
      }
      setSession(guidedSession);
      setInitialReview(null);
      setConfirmedInitialReviewIssues([]);
      if (guidedSession.phase !== "phase_two")
        setIsReviewingBasicInformation(false);
      setAssessments(response.assessments);
      if (replace) {
        if (draft) {
          const replacedFields = draft.field_metadata
            .filter((item) => item.source_field === field)
            .map((item) => item.field);
          setEditSource(null);
          setEditAnswer("");
          const refreshed = await loadDraft(guidedSession.id, false);
          if (refreshed)
            setProfile((current) =>
              syncProfileFields(current, refreshed, replacedFields),
            );
        }
      } else {
        setAnswer("");
        if (next.status === "ready_for_confirmation") {
          await loadDraft(guidedSession.id, true, basicInformation);
        }
      }
      return true;
    } catch (cause) {
      setError(messageFrom(cause));
      if (
        cause instanceof ApiClientError &&
        cause.status === 409 &&
        session.status === "ready_for_confirmation"
      ) {
        await loadDraft(session.id, false);
      }
      return false;
    } finally {
      setBusyAction(null);
    }
  }

  async function submitCurrentAnswer(value?: string) {
    if (!session?.next_question) {
      setError("当前问询状态已变化，请刷新后继续。");
      return;
    }
    const answerToSubmit =
      value ??
      (session.next_question.field === "basic_information"
        ? basicInformationAnswer(
            basicInformation,
            session.question_set_version >= 3,
          )
        : answer);
    if (!answerToSubmit) {
      setError("请填写必填资料后继续；第 3 版问询还需要身高和外观特征。");
      return;
    }
    await sendAnswer(session.next_question.field, answerToSubmit);
  }

  async function saveBasicInformationAndReturn() {
    const nextAnswer = basicInformationAnswer(
      basicInformation,
      session?.question_set_version !== undefined && session.question_set_version >= 3,
    );
    if (!nextAnswer) {
      setError("请填写姓名或称呼，或返回补充线索继续填写。");
      return;
    }
    const saved = await sendAnswer("basic_information", nextAnswer, true);
    if (saved) setIsReviewingBasicInformation(false);
  }

  async function confirmCase() {
    if (!token || !session || !draft) return;
    if (draft.confirmation_blocked_reasons.length > 0) {
      setError("当前存在阻断性核对项。请返回修改相关问询内容，再重新确认。");
      return;
    }
    if (!profile.display_name.trim() || !profile.last_seen_location.trim()) {
      setError("请先确认姓名或称呼，以及最后出现地点。");
      return;
    }
    if (!initialReview?.ready_for_second_confirmation) {
      setError("请先完成 AI 初步审核和家属疑点确认，再进行二次确认提交。");
      return;
    }

    setBusyAction("confirm");
    setError("");
    try {
      const response = await confirmIntakeSession(
        token,
        session.id,
        normalizedProfile(profile),
      );
      await onConfirmed(response.case_id, response.case_code);
      clearStoredState(storageKey);
    } catch (cause) {
      setError(messageFrom(cause));
      setConfirmReviewOpen(false);
      if (cause instanceof ApiClientError && cause.status === 409) {
        await loadDraft(session.id, false);
      }
    } finally {
      setBusyAction(null);
    }
  }

  async function uploadPhoto(file: File | undefined) {
    if (!token || !session || !file) return;
    if (!["image/jpeg", "image/png"].includes(file.type)) {
      setError("请上传 JPEG 或 PNG 格式的走失者照片。");
      return;
    }
    setBusyAction("photo");
    setError("");
    try {
      const photo = await uploadIntakePhoto(token, session.id, file);
      setPhotos((current) => [...current, photo]);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function restoreRevision(revision: IntakeAnswerRevision) {
    if (!token || !session) return;
    setBusyAction("replace");
    setError("");
    try {
      const response = await restoreIntakeAnswerRevision(
        token,
        session.id,
        revision.field,
        revision.id,
      );
      const nextSession = sessionFromAnswerResponse(response);
      setSession(nextSession);
      setInitialReview(null);
      setConfirmedInitialReviewIssues([]);
      const refreshed = await loadDraft(nextSession.id, true, basicInformation);
      if (refreshed) setProfile(profileFromDraft(refreshed, basicInformation));
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function startInitialReview() {
    if (!token || !session || !draft) return;
    if (draft.confirmation_blocked_reasons.length > 0) {
      setError("请先修正阻断性的规则核对项，再提交 AI 初步审核。");
      return;
    }
    setBusyAction("initial_review");
    setError("");
    setAiReviewStage("queued");
    activeAiExecutionRef.current = null;
    setActiveAiExecution(null);
    try {
      const review = await startIntakeAiInitialReview(
        token,
        session.id,
        normalizedProfile(profile),
        updateAiReviewStage,
      );
      setInitialReview(review);
      setConfirmedInitialReviewIssues([]);
      setSession((current) =>
        current ? { ...current, status: review.status } : current,
      );
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      if (!activeAiExecutionRef.current) setAiReviewStage(null);
      setBusyAction(null);
    }
  }

  async function generateDraftVersion() {
    if (!token || !session) return;
    setBusyAction("generate");
    setError("");
    setAiReviewStage("queued");
    activeAiExecutionRef.current = null;
    setActiveAiExecution(null);
    try {
      const next = await generateIntakeDraft(
        token,
        session.id,
        updateAiReviewStage,
      );
      setDraft(next);
      setProfile(profileFromDraft(next, basicInformation));
      setProfileVersions((current) => [next, ...current]);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      if (!activeAiExecutionRef.current) setAiReviewStage(null);
      setBusyAction(null);
    }
  }

  async function acknowledgeInitialReview() {
    if (!token || !session || !initialReview) return;
    if (confirmedInitialReviewIssues.length !== initialReview.issues.length) {
      setError("请逐项确认所有标注内容，或返回修改问询后重新初审。");
      return;
    }
    setBusyAction("acknowledge_initial_review");
    setError("");
    try {
      const review = await acknowledgeIntakeAiInitialReview(
        token,
        session.id,
        confirmedInitialReviewIssues,
      );
      setInitialReview(review);
      setSession((current) =>
        current ? { ...current, status: review.status } : current,
      );
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusyAction(null);
    }
  }

  function toggleInitialReviewIssue(issueId: string) {
    setConfirmedInitialReviewIssues((current) =>
      current.includes(issueId)
        ? current.filter((item) => item !== issueId)
        : [...current, issueId],
    );
  }

  function openSourceEditor(source: IntakeProfileDraftFieldMetadata) {
    const value =
      draft?.profile[source.field as keyof IntakeDraftProfile] ?? "";
    setEditSource(source);
    setEditAnswer(value ?? "");
    setConfirmReviewOpen(false);
    setError("");
  }

  function requestCancel() {
    if (answer.trim() && typeof window !== "undefined") {
      const proceed = window.confirm(
        "当前答案尚未提交。它会仅保留在此标签页草稿中；确定暂时离开吗？",
      );
      if (!proceed) return;
    }
    onCancel();
  }

  if (!session) {
    return (
      <section
        className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5"
        aria-labelledby="intake-start-title"
      >
        <span className="text-xs font-semibold text-brand-700">
          家属建档 · 规则化问询
        </span>
        <h2
          id="intake-start-title"
          className="mt-1 text-xl font-bold text-slate-950"
        >
          先整理信息，再由您确认建案
        </h2>
        <p className="max-w-2xl text-sm leading-6 text-slate-600">
          问询分阶段进行。家属提供的内容会先作为待确认草稿，系统不会将规则整理结果直接写成案件事实。
        </p>
        <ul className="mt-4 grid gap-2 text-sm text-slate-700 sm:grid-cols-3">
          <li className="rounded-md bg-slate-50 px-3 py-2">
            1. 分步填写，随时标记未知
          </li>
          <li className="rounded-md bg-slate-50 px-3 py-2">
            2. 查看来源与待核对项
          </li>
          <li className="rounded-md bg-slate-50 px-3 py-2">
            3. 人工确认后才创建案件
          </li>
        </ul>
        {error && <Alert>{error}</Alert>}
        <div className="mt-5 flex flex-wrap gap-2">
          <Button
            variant="primary"
            onPress={() => void begin()}
            isDisabled={!hasHydrated || isBusy}
          >
            {busyAction === "begin" && (
              <Spinner size="sm" aria-label="正在创建问询" />
            )}
            开始问询
          </Button>
          <Button variant="ghost" onPress={requestCancel} isDisabled={isBusy}>
            暂不开始
          </Button>
        </div>
      </section>
    );
  }

  if (draft) {
    return (
      <section
        className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5"
        aria-labelledby="intake-draft-title"
      >
        <header className="flex flex-col justify-between gap-3 border-b border-slate-100 pb-4 sm:flex-row sm:items-start">
          <div>
            <span className="text-xs font-semibold text-brand-700">
              第 3 步 · 人工确认
            </span>
            <h2
              id="intake-draft-title"
              className="mt-1 text-xl font-bold text-slate-950"
            >
              核对老人画像草稿
            </h2>
            <p className="mb-0 mt-1 text-sm leading-6 text-slate-600">
              每项内容均保持为草稿，只有您完成确认后才会创建正式案件。
            </p>
          </div>
          <Chip size="sm" variant="soft">
            <Chip.Label>需要人工确认</Chip.Label>
          </Chip>
        </header>

        <div
          className="mt-4 rounded-md border border-amber-200 bg-amber-50 px-3 py-3 text-sm leading-6 text-amber-950"
          role="status"
        >
          <div className="flex items-start gap-2">
            <ShieldCheck
              className="mt-0.5 shrink-0"
              size={17}
              aria-hidden="true"
            />
            <span>
              以下信息仅来自本次家属问询。请核对来源、时间与内容；未确认前，它们不是正式案件事实。
            </span>
          </div>
        </div>

        {error && <Alert>{error}</Alert>}
        <AssessmentList items={displayedAssessments} />
        <DraftProfileReview draft={draft} onEditSource={openSourceEditor} />
        {!editSource && (
          <section className="mt-5 rounded-md border border-slate-200 bg-slate-50 p-4">
            <div className="flex items-center justify-between gap-2">
              <h3 className="m-0 text-sm font-bold text-slate-950">
                AI 画像草稿版本
              </h3>
              <Button
                size="sm"
                variant="ghost"
                isDisabled={isBusy}
                onPress={() => void generateDraftVersion()}
              >
                {busyAction === "generate" && (
                  <Spinner size="sm" aria-label="正在生成新版本" />
                )}
                {busyAction === "generate" ? "正在生成新版本" : "生成新版本"}
              </Button>
            </div>
            {(busyAction === "generate" ||
              (activeAiExecution?.workflow === "intake_profile_draft" &&
                aiReviewStage === "failed")) && (
              <AiReviewProgress
                stage={aiReviewStage ?? "queued"}
                title="AI 画像草稿生成中"
              />
            )}
            {profileVersions.length === 0 ? (
              <p className="mb-0 mt-2 text-xs text-slate-600">
                尚未生成 AI 画像候选；当前内容仍来自家属原始回答。
              </p>
            ) : (
              <ul className="mb-0 mt-3 space-y-2 p-0">
                {profileVersions.slice(0, 8).map((version) => (
                  <li
                    key={version.id}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-slate-200 bg-white p-3"
                  >
                    <span className="text-xs text-slate-700">
                      v{version.version} · {version.status} ·{" "}
                      {version.degradation_status}
                    </span>
                    <div className="flex gap-2">
                      <Button
                        size="sm"
                        variant="ghost"
                        isDisabled={isBusy || version.status !== "draft"}
                        onPress={() =>
                          void (async () => {
                            if (!token || !session) return;
                            const updated = await reviewIntakeDraft(
                              token,
                              session.id,
                              version.id,
                              "confirm",
                              "family confirmed profile candidate",
                            );
                            setDraft(updated);
                            setProfileVersions((items) =>
                              items.map((item) =>
                                item.id === updated.id ? updated : item,
                              ),
                            );
                          })()
                        }
                      >
                        确认
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        isDisabled={isBusy}
                        onPress={() =>
                          void (async () => {
                            if (!token || !session) return;
                            const restored = await restoreIntakeDraft(
                              token,
                              session.id,
                              version.id,
                              "family restored this candidate for another review",
                            );
                            setDraft(restored);
                            setProfile(
                              profileFromDraft(restored, basicInformation),
                            );
                            setProfileVersions((items) => [restored, ...items]);
                          })()
                        }
                      >
                        恢复
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
        )}
        {!editSource && profileVersions.length > 0 && (
          <section
            className="mt-3 rounded-md border border-slate-200 bg-white p-4"
            aria-label="画像草稿版本操作"
          >
            <h3 className="m-0 text-sm font-bold text-slate-950">
              版本确认、拒绝与比较
            </h3>
            <div className="mt-3 grid gap-2">
              {profileVersions.slice(0, 8).map((version) => (
                <div
                  key={`actions-${version.id}`}
                  className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 py-2 last:border-0"
                >
                  <label className="flex items-center gap-2 text-xs text-slate-700">
                    <input
                      type="checkbox"
                      checked={
                        comparison?.from === version.id ||
                        comparison?.to === version.id
                      }
                      onChange={() =>
                        setComparison((current) =>
                          current?.from === version.id
                            ? { ...current, from: "" }
                            : current?.to === version.id
                              ? { ...current, to: "" }
                              : !current?.from
                                ? { from: version.id, to: "", fields: [] }
                                : {
                                    from: current.from,
                                    to: version.id,
                                    fields: [],
                                  },
                        )
                      }
                    />
                    v{version.version}
                  </label>
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={isBusy || version.status !== "draft"}
                      onPress={() =>
                        void (async () => {
                          if (!token || !session) return;
                          const updated = await reviewIntakeDraft(
                            token,
                            session.id,
                            version.id,
                            "reject",
                            "family rejected profile candidate",
                          );
                          setProfileVersions((items) =>
                            items.map((item) =>
                              item.id === updated.id ? updated : item,
                            ),
                          );
                        })()
                      }
                    >
                      拒绝
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={isBusy}
                      onPress={() =>
                        void (async () => {
                          if (!token || !session) return;
                          const restored = await restoreIntakeDraft(
                            token,
                            session.id,
                            version.id,
                            "family restored this candidate for another review",
                          );
                          setDraft(restored);
                          setProfile(
                            profileFromDraft(restored, basicInformation),
                          );
                          setProfileVersions((items) => [restored, ...items]);
                        })()
                      }
                    >
                      恢复
                    </Button>
                  </div>
                </div>
              ))}
            </div>
            <Button
              className="mt-3"
              size="sm"
              variant="secondary"
              isDisabled={
                !token ||
                !session ||
                !comparison?.from ||
                !comparison?.to ||
                comparison.from === comparison.to ||
                isBusy
              }
              onPress={() =>
                void (async () => {
                  if (
                    !token ||
                    !session ||
                    !comparison?.from ||
                    !comparison?.to
                  )
                    return;
                  const diff = await diffIntakeDraftVersions(
                    token,
                    session.id,
                    comparison.from,
                    comparison.to,
                  );
                  setComparison({
                    from: comparison.from,
                    to: comparison.to,
                    fields: diff.changed_fields,
                  });
                })()
              }
            >
              比较所选版本
            </Button>
            {comparison?.fields.length ? (
              <p className="mb-0 mt-2 text-xs text-slate-700">
                变化字段：{comparison.fields.join("、")}
              </p>
            ) : comparison?.from && comparison?.to ? (
              <p className="mb-0 mt-2 text-xs text-slate-500">
                选择比较后将显示变更字段；无结果表示字段值一致。
              </p>
            ) : null}
          </section>
        )}
        {answerRevisions.length > 0 && !editSource && (
          <section
            className="mt-5 rounded-md border border-slate-200 bg-slate-50 p-4"
            aria-labelledby="answer-history-title"
          >
            <h3
              id="answer-history-title"
              className="m-0 text-sm font-bold text-slate-950"
            >
              问询修订历史
            </h3>
            <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
              恢复旧版本会使当前 AI 初审失效，系统会重新生成画像并要求再次初审。
            </p>
            <ul className="mb-0 mt-3 space-y-2 p-0">
              {answerRevisions
                .slice()
                .reverse()
                .slice(0, 12)
                .map((revision) => (
                  <li
                    key={revision.id}
                    className="flex flex-col gap-2 rounded-md border border-slate-200 bg-white p-3 sm:flex-row sm:items-center sm:justify-between"
                  >
                    <div className="min-w-0">
                      <strong className="text-xs text-slate-900">
                        {questionLabels[revision.field] ?? revision.field} ·{" "}
                        {revision.revision_kind}
                      </strong>
                      <p className="mb-0 mt-1 line-clamp-2 text-xs leading-5 text-slate-600">
                        {revision.answer}
                      </p>
                    </div>
                    <Button
                      size="sm"
                      variant="ghost"
                      isDisabled={isBusy}
                      onPress={() => void restoreRevision(revision)}
                    >
                      恢复此版本
                    </Button>
                  </li>
                ))}
            </ul>
          </section>
        )}

        {draft.direction_hypotheses.length > 0 && (
          <section
            className="mt-5"
            aria-labelledby="direction-hypotheses-title"
          >
            <h3
              id="direction-hypotheses-title"
              className="text-sm font-bold text-slate-950"
            >
              待核实方向
            </h3>
            <div className="grid gap-3 md:grid-cols-2">
              {draft.direction_hypotheses.map((item, index) => (
                <article
                  key={`${item.generated_at}-${index}`}
                  className="rounded-md border border-slate-200 bg-slate-50 p-3 text-sm"
                >
                  <strong className="text-slate-900">可能方向（不确定）</strong>
                  <p className="mb-1 mt-2 leading-6 text-slate-700">
                    {item.description}
                  </p>
                  <p className="m-0 text-xs leading-5 text-slate-600">
                    {item.uncertainty_notice}
                  </p>
                </article>
              ))}
            </div>
          </section>
        )}

        {editSource && (
          <section
            className="mt-5 rounded-md border border-brand-100 bg-brand-50 p-4"
            aria-labelledby="source-editor-title"
          >
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <span className="text-xs font-semibold text-brand-700">
                  返回问询修改
                </span>
                <h3
                  id="source-editor-title"
                  className="m-0 text-base font-bold text-slate-950"
                >
                  修改“
                  {questionLabels[editSource.source_field] ??
                    editSource.source_field}
                  ”
                </h3>
              </div>
              <Button
                size="sm"
                variant="ghost"
                onPress={() => setEditSource(null)}
                isDisabled={isBusy}
              >
                取消修改
              </Button>
            </div>
            <Field label="修订后的家属回答" required>
              <TextArea
                value={editAnswer}
                rows={4}
                maxLength={2000}
                onChange={(event) => setEditAnswer(event.target.value)}
                fullWidth
              />
            </Field>
            <div className="mt-3 flex flex-wrap gap-2">
              <Button
                variant="secondary"
                onPress={() =>
                  void sendAnswer(editSource.source_field, editAnswer, true)
                }
                isDisabled={isBusy}
              >
                {busyAction === "replace" && (
                  <Spinner size="sm" aria-label="正在保存修订" />
                )}
                保存修订并刷新草稿
              </Button>
              <Button
                variant="ghost"
                onPress={() =>
                  void sendAnswer(editSource.source_field, "未知", true)
                }
                isDisabled={isBusy}
              >
                标记为未知
              </Button>
            </div>
          </section>
        )}

        {!editSource && (
          <section
            className="mt-5 border-t border-slate-200 pt-5"
            aria-labelledby="confirmed-profile-title"
          >
            <div className="flex items-start gap-2">
              <CheckCircle2
                className="mt-0.5 shrink-0 text-brand-700"
                size={18}
                aria-hidden="true"
              />
              <div>
                <h3
                  id="confirmed-profile-title"
                  className="m-0 text-base font-bold text-slate-950"
                >
                  确认后写入案件的资料
                </h3>
                <p className="mb-0 mt-1 text-sm leading-6 text-slate-600">
                  请在这里修订正式资料。带 * 的两项为当前服务端真正必填内容。
                </p>
              </div>
            </div>
            <ProfileForm
              profile={profile}
              onChange={(nextProfile) => {
                setProfile(nextProfile);
                setInitialReview(null);
                setConfirmedInitialReviewIssues([]);
                setAiReviewStage(null);
              }}
            />
            <InitialReviewPanel
              review={initialReview}
              isReviewing={
                busyAction === "initial_review" ||
                (activeAiExecution?.workflow === "intake_initial_review" &&
                  aiReviewStage === "failed")
              }
              stage={aiReviewStage}
              confirmedIssueIds={confirmedInitialReviewIssues}
              onToggleIssue={toggleInitialReviewIssue}
            />
            {confirmReviewOpen && (
              <div
                ref={confirmDialogRef}
                tabIndex={-1}
                className="mt-4 rounded-md border border-brand-100 bg-brand-50 p-4"
                role="alertdialog"
                aria-labelledby="confirm-case-title"
              >
                <h4
                  id="confirm-case-title"
                  className="m-0 text-sm font-bold text-slate-950"
                >
                  确认创建案件？
                </h4>
                <p className="mb-3 mt-1 text-sm leading-6 text-slate-700">
                  创建后将生成正式案件。问询草稿会保留为本次确认的依据，但不会替代您刚刚核对的资料。
                </p>
                <div className="flex flex-wrap gap-2">
                  <Button
                    variant="primary"
                    onPress={() => void confirmCase()}
                    isDisabled={
                      isBusy || draft.confirmation_blocked_reasons.length > 0
                    }
                  >
                    {busyAction === "confirm" && (
                      <Spinner size="sm" aria-label="正在创建案件" />
                    )}
                    确认并创建案件
                  </Button>
                  <Button
                    variant="ghost"
                    onPress={() => setConfirmReviewOpen(false)}
                    isDisabled={isBusy}
                  >
                    返回编辑
                  </Button>
                </div>
              </div>
            )}
            <div className="mt-5 flex flex-wrap gap-2">
              <Button
                variant="primary"
                onPress={() => {
                  if (!initialReview) {
                    void startInitialReview();
                  } else if (initialReview.requires_family_acknowledgement) {
                    void acknowledgeInitialReview();
                  } else {
                    void confirmCase();
                  }
                }}
                isDisabled={
                  isBusy ||
                  confirmReviewOpen ||
                  draft.confirmation_blocked_reasons.length > 0 ||
                  (initialReview?.requires_family_acknowledgement === true &&
                    confirmedInitialReviewIssues.length !==
                      initialReview.issues.length)
                }
              >
                {busyAction === "initial_review" && (
                  <Spinner size="sm" aria-label="正在进行 AI 初步审核" />
                )}
                {busyAction === "acknowledge_initial_review" && (
                  <Spinner size="sm" aria-label="正在确认初审标注" />
                )}
                {busyAction === "confirm" && (
                  <Spinner size="sm" aria-label="正在提交指挥端" />
                )}
                {!initialReview
                  ? "首次确认并进行 AI 初步审核"
                  : initialReview.requires_family_acknowledgement
                    ? "确认初审标注"
                    : "二次确认并提交指挥端"}
              </Button>
              <Button
                variant="ghost"
                onPress={requestCancel}
                isDisabled={isBusy}
              >
                暂不确认
              </Button>
            </div>
          </section>
        )}

        {!editSource && sourceOptions.length > 0 && (
          <section className="mt-5 border-t border-slate-200 pt-4">
            <h3 className="m-0 text-sm font-bold text-slate-950">
              需要补充或修改问询？
            </h3>
            <p className="mb-3 mt-1 text-xs leading-5 text-slate-600">
              修订会作为新的草稿答案保存，并刷新当前画像与核对项。
            </p>
            <div className="flex flex-wrap gap-2">
              {sourceOptions.map((source) => (
                <Button
                  key={source.source_field}
                  size="sm"
                  variant="ghost"
                  onPress={() => openSourceEditor(source)}
                  isDisabled={isBusy}
                >
                  <FilePenLine size={15} aria-hidden="true" /> 修改
                  {questionLabels[source.source_field] ?? source.source_field}
                </Button>
              ))}
            </div>
          </section>
        )}
      </section>
    );
  }

  const question = session.next_question;
  const isReviewingPhaseOne =
    session.phase === "phase_two" && isReviewingBasicInformation;
  const completed = session.completed_phase_one_fields.length;
  const isReportDetailsSession = session.question_set_version >= 3;
  const phaseTotal = isReportDetailsSession ? 5 : 2;
  const currentLabel = isReviewingPhaseOne
    ? "基本情况（编辑）"
    : question
      ? (questionLabels[question.field] ?? question.field)
      : "正在整理草稿";

  return (
    <section
      className="border-y border-slate-200 bg-white px-4 py-6 sm:px-5"
      aria-labelledby="intake-question-title"
    >
      <header className="border-b border-slate-100 pb-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <span className="text-xs font-semibold text-brand-700">
              {isReviewingPhaseOne || session.phase === "phase_one"
                ? "第 1 步 · 基本情况"
                : "第 2 步 · 补充线索"}
            </span>
            <h2
              id="intake-question-title"
              className="mt-1 text-xl font-bold text-slate-950"
            >
              {currentLabel}
            </h2>
          </div>
          <Chip size="sm" variant="soft">
            <Chip.Label>
              {session.guidance_mode === "rule_based" ? "规则化问询" : "问询中"}
            </Chip.Label>
          </Chip>
        </div>
        <div className="mt-4" aria-label="问询进度">
          <div className="flex items-center justify-between text-xs text-slate-600">
            <span>
              第一阶段已填写 {completed} / {phaseTotal} 项
            </span>
            <span>
              {session.phase_transition_ready
                ? "必要信息已齐全"
                : `仍缺：${session.missing_phase_one_fields.map(questionLabel).join("、")}`}
            </span>
          </div>
          <div className="mt-2 h-2 overflow-hidden rounded-full bg-slate-100">
            <div
              className="h-full rounded-full bg-brand-600 transition-[width] duration-200 motion-reduce:transition-none"
              style={{
                width: `${Math.min(100, (completed / phaseTotal) * 100)}%`,
              }}
            />
          </div>
        </div>
      </header>

      {isFetchingAiFollowUp && (
        <AiReviewProgress
          stage="generating"
          title="正在准备下一项问询"
        />
      )}
      {error && <Alert>{error}</Alert>}
      <AssessmentList items={displayedAssessments} />

      {isReportDetailsSession && <section
        className="mt-5 rounded-md border border-brand-200 bg-brand-50 p-4"
        aria-labelledby="intake-photo-title"
      >
        <h3 id="intake-photo-title" className="m-0 text-sm font-bold text-slate-950">
          走失者照片 <span aria-hidden="true">*</span>
        </h3>
        <p id="intake-photo-help" className="mb-3 mt-1 text-xs leading-5 text-slate-700">
          请上传至少一张近期 JPEG 或 PNG 照片。照片仅用于受控案件处理，不会公开展示、用于人脸识别或写入 AI 审核日志。
        </p>
        <input
          type="file"
          accept="image/jpeg,image/png"
          aria-describedby="intake-photo-help"
          aria-label="上传走失者照片"
          disabled={isBusy || photos.length >= 4}
          onChange={(event) => {
            void uploadPhoto(event.target.files?.[0]);
            event.currentTarget.value = "";
          }}
          className="block w-full text-sm text-slate-700 file:mr-3 file:rounded-md file:border-0 file:bg-brand-700 file:px-3 file:py-2 file:text-sm file:font-semibold file:text-white hover:file:bg-brand-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-600 focus-visible:ring-offset-2 disabled:opacity-60"
        />
        {busyAction === "photo" && (
          <p className="mb-0 mt-2 text-xs text-slate-700" role="status">
            正在安全处理照片…
          </p>
        )}
        {photos.length > 0 ? (
          <ul className="mb-0 mt-3 space-y-1 p-0 text-xs text-slate-700" aria-live="polite">
            {photos.map((photo) => (
              <li key={photo.id} className="list-none rounded bg-white px-2 py-1">
                已上传：{photo.original_filename}（{Math.ceil(photo.byte_size / 1024)} KB）
              </li>
            ))}
          </ul>
        ) : (
          <p className="mb-0 mt-3 text-xs text-amber-800" role="status">
            创建案件前必须至少上传一张照片。
          </p>
        )}
      </section>}

      {question ? (
        <form
          className="mt-5"
          onSubmit={(event) => {
            event.preventDefault();
            if (isReviewingPhaseOne) {
              void saveBasicInformationAndReturn();
              return;
            }
            void submitCurrentAnswer();
          }}
        >
          {isReviewingPhaseOne || question.field === "basic_information" ? (
            <BasicInformationForm
              value={basicInformation}
              onChange={setBasicInformation}
              required={isReviewingPhaseOne || question.required}
              requiresReportDetails={isReportDetailsSession}
              hint={
                isReviewingPhaseOne
                  ? "可在这里更正基本情况；保存后将返回补充线索。"
                  : questionReasons[question.field]
              }
            />
          ) : question.field === "police_report_status" ? (
            <Field label="是否报警" required hint="请选择实际情况；系统不会自动判断报警状态。">
              <select
                aria-label="是否报警"
                value={answer}
                onChange={(event) => setAnswer(event.target.value)}
                className="h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-brand-600 focus:ring-2 focus:ring-brand-100"
              >
                <option value="">请选择</option>
                <option value="已报警">已报警</option>
                <option value="未报警">未报警</option>
                <option value="不清楚">不清楚</option>
              </select>
            </Field>
          ) : question.field === "family_phone" ? (
            <Field label="家属电话" required hint="仅向经授权的家属和指挥人员开放，不会出现在公开进展中。">
              <Input
                type="tel"
                autoComplete="tel"
                inputMode="tel"
                value={answer}
                onChange={(event) => setAnswer(event.target.value)}
                maxLength={40}
                fullWidth
              />
            </Field>
          ) : (
            <Field
              label={
                legacyDefaultQuestionPrompts[question.field] === question.prompt
                  ? defaultQuestionPrompts[question.field]
                  : question.prompt
              }
              required={question.required}
              hint={questionReasons[question.field]}
            >
              <TextArea
                value={answer}
                onChange={(event) => setAnswer(event.target.value)}
                rows={5}
                maxLength={2000}
                fullWidth
              />
            </Field>
          )}
          <p className="mt-2 text-xs leading-5 text-slate-500">
            仅在当前浏览器标签页暂存尚未提交的答案；不会写入
            URL、日志或跨设备存储。
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            {session.phase === "phase_two" && !isReviewingPhaseOne && (
              <Button
                variant="ghost"
                onPress={() => {
                  setIsReviewingBasicInformation(true);
                  setError("");
                }}
                isDisabled={isBusy}
              >
                <ArrowLeft size={16} aria-hidden="true" />
                上一步
              </Button>
            )}
            <Button type="submit" variant="primary" isDisabled={isBusy}>
              {(busyAction === "answer" ||
                (isReviewingPhaseOne && busyAction === "replace")) && (
                <Spinner
                  size="sm"
                  aria-label={
                    isReviewingPhaseOne
                      ? "正在保存基本情况更正"
                      : "正在保存答案"
                  }
                />
              )}
              {isReviewingPhaseOne ? "保存更正并返回补充线索" : "保存并继续"}
            </Button>
            {isReviewingPhaseOne && (
              <Button
                variant="ghost"
                onPress={() => setIsReviewingBasicInformation(false)}
                isDisabled={isBusy}
              >
                返回补充线索（不保存）
              </Button>
            )}
            <Button
              type="button"
              variant="ghost"
              onPress={() => void submitCurrentAnswer("未知")}
              isDisabled={isBusy}
            >
              标记为未知
            </Button>
            <Button
              type="button"
              variant="ghost"
              onPress={requestCancel}
              isDisabled={isBusy}
            >
              <ArrowLeft size={16} aria-hidden="true" />
              暂时离开
            </Button>
          </div>
        </form>
      ) : (
        <div className="mt-5 rounded-md border border-slate-200 bg-slate-50 p-4 text-sm text-slate-700">
          <div className="flex items-center gap-2">
            <Spinner size="sm" />
            <span>问询已完成，正在获取需要人工确认的画像草稿。</span>
          </div>
          <Button
            className="mt-3"
            size="sm"
            variant="ghost"
            onPress={() => void loadDraft(session.id, true)}
            isDisabled={isBusy}
          >
            重新获取草稿
          </Button>
        </div>
      )}
    </section>
  );
}

function InitialReviewPanel({
  review,
  isReviewing,
  stage,
  confirmedIssueIds,
  onToggleIssue,
}: {
  review: IntakeAiInitialReviewResponse | null;
  isReviewing: boolean;
  stage: AiReviewStage | null;
  confirmedIssueIds: string[];
  onToggleIssue: (issueId: string) => void;
}) {
  if (!review) {
    return (
      <div
        className="mt-4 rounded-md border border-brand-100 bg-brand-50 p-3 text-sm leading-6 text-slate-700"
        role={isReviewing ? "status" : undefined}
      >
        {isReviewing ? (
          <AiReviewProgress
            stage={stage ?? "queued"}
            title="AI 初步审核进行中"
          />
        ) : (
          "首次确认后，系统会进行 AI 初步审核。它只标注需要您核对的疑点，不能确认事实、修改资料或判断位置。"
        )}
      </div>
    );
  }

  return (
    <section
      className="mt-4 rounded-md border border-amber-200 bg-amber-50 p-4"
      aria-labelledby="initial-review-title"
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h3
            id="initial-review-title"
            className="m-0 text-base font-bold text-slate-950"
          >
            AI 初步审核结果
          </h3>
          <p className="mb-0 mt-1 text-sm leading-6 text-slate-700">
            {review.degradation_status === "rule_based_fallback"
              ? "AI 服务不可用或输出未通过校验，已使用规则一致性检查回退。"
              : "以下内容仅是待家属确认的提醒，不是事实结论、位置判断或行动指令。"}
          </p>
        </div>
        <Chip size="sm" variant="soft">
          <Chip.Label>
            {review.ready_for_second_confirmation
              ? "已完成家属确认"
              : "等待家属确认"}
          </Chip.Label>
        </Chip>
      </div>

      {review.blocking_assessments.length > 0 && (
        <div className="mt-3 rounded-md border border-red-200 bg-red-50 p-3 text-sm leading-6 text-red-900">
          <strong>需要先修正的规则核对项</strong>
          {review.blocking_assessments.map((item) => (
            <p
              key={`${item.field_path}-${item.conflict_type}`}
              className="mb-0 mt-2"
            >
              {item.evidence_summary} {item.suggested_action}
            </p>
          ))}
        </div>
      )}

      {review.issues.length === 0 ? (
        <p className="mb-0 mt-3 text-sm leading-6 text-slate-700">
          未发现需要额外确认的 AI
          疑点。请确认后进行二次提交；这不代表资料已经被系统认定为真实无误。
        </p>
      ) : (
        <div className="mt-3 grid gap-3">
          {review.issues.map((item) => {
            const checked = confirmedIssueIds.includes(item.id);
            return (
              <label
                key={item.id}
                className="flex cursor-pointer gap-3 rounded-md border border-amber-200 bg-white p-3 text-sm leading-6 text-slate-700"
              >
                <input
                  className="mt-1 h-4 w-4 shrink-0"
                  type="checkbox"
                  checked={checked}
                  disabled={review.ready_for_second_confirmation}
                  onChange={() => onToggleIssue(item.id)}
                />
                <span>
                  <strong className="block text-slate-950">
                    {questionLabels[item.field] ?? item.field}
                  </strong>
                  <span className="block">{item.evidence_summary}</span>
                  <span className="mt-1 block text-slate-900">
                    请确认：{item.clarification_question}
                  </span>
                </span>
              </label>
            );
          })}
        </div>
      )}
    </section>
  );
}

function DraftProfileReview({
  draft,
  onEditSource,
}: {
  draft: IntakeDraft;
  onEditSource: (source: IntakeProfileDraftFieldMetadata) => void;
}) {
  const metadata = new Map(
    draft.field_metadata.map((item) => [item.field, item]),
  );
  const fields: Array<{ field: keyof IntakeDraftProfile; label: string }> = [
    { field: "physical_description", label: "体貌描述" },
    { field: "clothing_description", label: "衣着与随身物品" },
    { field: "health_notes", label: "健康注意事项" },
    { field: "mobility_notes", label: "行动与移动" },
    { field: "transportation_ability", label: "出行能力" },
    { field: "frequent_locations", label: "常去地点" },
    { field: "last_seen_information", label: "最后出现信息" },
    { field: "behavior_habits", label: "行为习惯" },
    { field: "suspicious_motive", label: "可疑动机" },
  ];

  return (
    <section className="mt-5" aria-labelledby="draft-profile-title">
      <h3
        id="draft-profile-title"
        className="m-0 text-base font-bold text-slate-950"
      >
        问询整理出的画像草稿
      </h3>
      <div className="mt-3 grid gap-3 md:grid-cols-2">
        {fields.map(({ field, label }) => {
          const value = draft.profile[field];
          const source = metadata.get(field);
          return (
            <article
              key={field}
              className="rounded-md border border-slate-200 bg-white p-4"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <h4 className="m-0 text-sm font-bold text-slate-950">
                  {label}
                </h4>
                <Chip size="sm" variant="soft">
                  <Chip.Label>草稿</Chip.Label>
                </Chip>
              </div>
              <p className="mb-3 mt-3 min-h-12 whitespace-pre-wrap text-sm leading-6 text-slate-700">
                {value ?? "尚未提供"}
              </p>
              {source ? (
                <div className="border-t border-slate-100 pt-3 text-xs leading-5 text-slate-600">
                  <span className="block">
                    来源：
                    {source.source === "family_provided"
                      ? "家属提供"
                      : "待核实提取"}{" "}
                    ·{" "}
                    {questionLabels[source.source_field] ?? source.source_field}
                  </span>
                  <span className="block">
                    生成于：{formatDate(source.generated_at)} · 状态：需人工确认
                  </span>
                  <Button
                    className="mt-2"
                    size="sm"
                    variant="ghost"
                    onPress={() => onEditSource(source)}
                  >
                    <FilePenLine size={14} aria-hidden="true" />
                    返回修改
                  </Button>
                </div>
              ) : (
                <p className="m-0 border-t border-slate-100 pt-3 text-xs text-slate-500">
                  暂无可核对的来源记录
                </p>
              )}
            </article>
          );
        })}
      </div>
    </section>
  );
}

function ProfileForm({
  profile,
  onChange,
}: {
  profile: ConfirmedIntakeProfile;
  onChange: (next: ConfirmedIntakeProfile) => void;
}) {
  return (
    <div className="mt-4 grid gap-3 sm:grid-cols-2">
      <Field label="姓名或称呼" required>
        <Input
          value={profile.display_name}
          maxLength={120}
          onChange={(event) =>
            onChange({ ...profile, display_name: event.target.value })
          }
          fullWidth
        />
      </Field>
      <Field label="最后出现地点" required>
        <Input
          value={profile.last_seen_location}
          onChange={(event) =>
            onChange({ ...profile, last_seen_location: event.target.value })
          }
          fullWidth
        />
      </Field>
      <Field label="年龄">
        <Input
          type="number"
          min={0}
          max={130}
          value={profile.age ?? ""}
          onChange={(event) =>
            onChange({
              ...profile,
              age: event.target.value ? Number(event.target.value) : null,
            })
          }
          fullWidth
        />
      </Field>
      <Field label="性别">
        <Input
          value={profile.gender ?? ""}
          onChange={(event) =>
            onChange({ ...profile, gender: nullable(event.target.value) })
          }
          fullWidth
        />
      </Field>
      <Field label="最后出现时间">
        <Input
          type="datetime-local"
          value={toDateTimeLocal(profile.last_seen_at)}
          onChange={(event) =>
            onChange({
              ...profile,
              last_seen_at: event.target.value
                ? new Date(event.target.value).toISOString()
                : null,
            })
          }
          fullWidth
        />
      </Field>
      <div className="hidden sm:block" aria-hidden="true" />
      <Field label="体貌描述">
        <TextArea
          value={profile.physical_description ?? ""}
          onChange={(event) =>
            onChange({
              ...profile,
              physical_description: nullable(event.target.value),
            })
          }
          rows={3}
          fullWidth
        />
      </Field>
      <Field label="衣着描述">
        <TextArea
          value={profile.clothing_description ?? ""}
          onChange={(event) =>
            onChange({
              ...profile,
              clothing_description: nullable(event.target.value),
            })
          }
          rows={3}
          fullWidth
        />
      </Field>
      <div className="sm:col-span-2">
        <Field label="健康注意事项">
          <TextArea
            value={profile.health_notes ?? ""}
            onChange={(event) =>
              onChange({
                ...profile,
                health_notes: nullable(event.target.value),
              })
            }
            rows={3}
            fullWidth
          />
        </Field>
      </div>
    </div>
  );
}

function BasicInformationForm({
  value,
  onChange,
  required,
  requiresReportDetails,
  hint,
}: {
  value: BasicInformationDraft;
  onChange: (next: BasicInformationDraft) => void;
  required: boolean;
  requiresReportDetails: boolean;
  hint?: string;
}) {
  return (
    <fieldset className="rounded-lg border border-slate-200 bg-slate-50/70 p-4">
      <legend className="px-1 text-sm font-semibold text-slate-800">
        基本信息{required ? <span aria-hidden="true"> *</span> : null}
      </legend>
      {hint && (
        <p className="mb-4 mt-1 flex items-start gap-1 text-xs leading-5 text-slate-500">
          <CircleHelp
            className="mt-0.5 shrink-0"
            size={14}
            aria-hidden="true"
          />
          {hint}
        </p>
      )}
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="姓名或称呼" required>
          <Input
            value={value.name}
            maxLength={120}
            autoComplete="name"
            onChange={(event) =>
              onChange({ ...value, name: event.target.value })
            }
            fullWidth
          />
        </Field>
        <Field label="性别">
          <select
            aria-label="性别"
            value={value.gender}
            onChange={(event) =>
              onChange({ ...value, gender: event.target.value })
            }
            className="h-10 w-full rounded-md border border-slate-300 bg-white px-3 text-sm text-slate-900 outline-none transition focus:border-brand-600 focus:ring-2 focus:ring-brand-100"
          >
            <option value="">暂不确定</option>
            <option value="男">男</option>
            <option value="女">女</option>
            <option value="其他">其他</option>
          </select>
        </Field>
        <Field label="年龄">
          <Input
            type="number"
            inputMode="numeric"
            min={0}
            max={130}
            value={value.age}
            onChange={(event) =>
              onChange({ ...value, age: event.target.value })
            }
            fullWidth
          />
        </Field>
        <Field label="身高（厘米）" required={requiresReportDetails}>
          <Input
            type="number"
            inputMode="numeric"
            min={30}
            max={250}
            value={value.height}
            onChange={(event) =>
              onChange({ ...value, height: event.target.value })
            }
            fullWidth
          />
        </Field>
        <div className="sm:col-span-2">
          <Field label="便于识别的外观特征" required={requiresReportDetails}>
            <TextArea
              value={value.appearance}
              onChange={(event) =>
                onChange({ ...value, appearance: event.target.value })
              }
              rows={3}
              maxLength={300}
              fullWidth
            />
          </Field>
        </div>
      </div>
    </fieldset>
  );
}

function AssessmentList({ items }: { items: IntakeAssessment[] }) {
  if (items.length === 0) return null;
  return (
    <section className="mt-4 space-y-2" aria-label="规则核对结果">
      {items.map((item, index) => (
        <div
          key={`${item.field_path}-${item.conflict_type}-${index}`}
          className={`rounded-md border px-3 py-3 text-sm ${assessmentClass(item.severity)}`}
        >
          <div className="flex items-start gap-2">
            <AlertTriangle
              className="mt-0.5 shrink-0"
              size={16}
              aria-hidden="true"
            />
            <div>
              <strong>{assessmentLabel(item.severity)}</strong>
              <span className="ml-2">{item.evidence_summary}</span>
              <p className="mb-0 mt-1 text-xs leading-5">
                {item.suggested_action}
              </p>
            </div>
          </div>
        </div>
      ))}
    </section>
  );
}

function Field({
  label,
  hint,
  required,
  children,
}: {
  label: string;
  hint?: string;
  required?: boolean;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm font-semibold text-slate-800">
        {label}
        {required ? <span aria-hidden="true"> *</span> : null}
      </span>
      {hint && (
        <span className="mb-2 flex items-start gap-1 text-xs leading-5 text-slate-500">
          <CircleHelp
            className="mt-0.5 shrink-0"
            size={14}
            aria-hidden="true"
          />
          {hint}
        </span>
      )}
      {children}
    </label>
  );
}

function Alert({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-3 text-sm leading-6 text-red-800"
      role="alert"
    >
      {children}
    </div>
  );
}

function uniqueSourceOptions(draft: IntakeDraft | null) {
  if (!draft) return [];
  const seen = new Set<string>();
  return draft.field_metadata.filter((item) => {
    if (seen.has(item.source_field)) return false;
    seen.add(item.source_field);
    return true;
  });
}

function profileFromDraft(
  draft: IntakeDraft,
  basicInformation: BasicInformationDraft,
): ConfirmedIntakeProfile {
  return {
    ...blankProfile,
    display_name: basicInformation.name.trim(),
    age: validInteger(basicInformation.age, 0, 130),
    gender: nullable(basicInformation.gender),
    physical_description:
      basicInformationDescription(basicInformation) ??
      draft.profile.physical_description,
    clothing_description: draft.profile.clothing_description,
    health_notes: draft.profile.health_notes,
    last_seen_location: draft.profile.last_seen_information ?? "",
  };
}

function sessionFromAnswerResponse(
  response: SubmitIntakeAnswerResponse,
): IntakeSession {
  return {
    id: response.session_id,
    question_set_version: response.question_set_version,
    status: response.status,
    missing_fields: response.missing_fields,
    phase: response.phase,
    completed_phase_one_fields: response.completed_phase_one_fields,
    missing_phase_one_fields: response.missing_phase_one_fields,
    phase_transition_ready: response.phase_transition_ready,
    next_question: response.next_question,
    guidance_mode: response.guidance_mode,
    ai_initial_review_status: response.ai_initial_review_status,
    privacy_notice: response.privacy_notice,
  };
}

function basicInformationAnswer(
  value: BasicInformationDraft,
  requiresReportDetails = false,
): string | null {
  const name = value.name.trim();
  if (!name) return null;
  const age = validInteger(value.age, 0, 130);
  const height = validInteger(value.height, 30, 250);
  if (requiresReportDetails && (height === null || !value.appearance.trim())) {
    return null;
  }
  const lines = [`姓名或称呼：${name}`];
  if (value.gender) lines.push(`性别：${value.gender}`);
  if (age !== null) lines.push(`年龄：${age} 岁`);
  if (height !== null) lines.push(`身高：${height} 厘米`);
  if (value.appearance.trim())
    lines.push(`外观特征：${value.appearance.trim()}`);
  return lines.join("\n");
}

function basicInformationDescription(
  value: BasicInformationDraft,
): string | null {
  const height = validInteger(value.height, 30, 250);
  const lines = [
    height === null ? null : `身高约 ${height} 厘米`,
    value.appearance.trim() || null,
  ].filter((item): item is string => item !== null);
  return lines.length > 0 ? lines.join("；") : null;
}

function validInteger(
  value: string,
  minimum: number,
  maximum: number,
): number | null {
  const normalized = value.trim();
  if (!normalized) return null;
  const number = Number(normalized);
  return Number.isInteger(number) && number >= minimum && number <= maximum
    ? number
    : null;
}

function syncProfileFields(
  current: ConfirmedIntakeProfile,
  draft: IntakeDraft,
  replacedFields: string[],
) {
  const next = { ...current };
  for (const field of replacedFields) {
    switch (field) {
      case "physical_description":
        next.physical_description = draft.profile.physical_description;
        break;
      case "clothing_description":
        next.clothing_description = draft.profile.clothing_description;
        break;
      case "health_notes":
        next.health_notes = draft.profile.health_notes;
        break;
      case "last_seen_information":
        next.last_seen_location = draft.profile.last_seen_information ?? "";
        break;
    }
  }
  return next;
}

function normalizedProfile(
  profile: ConfirmedIntakeProfile,
): ConfirmedIntakeProfile {
  return {
    ...profile,
    display_name: profile.display_name.trim(),
    last_seen_location: profile.last_seen_location.trim(),
    gender: nullable(profile.gender ?? ""),
    physical_description: nullable(profile.physical_description ?? ""),
    clothing_description: nullable(profile.clothing_description ?? ""),
    health_notes: nullable(profile.health_notes ?? ""),
  };
}

function toStoredSession(session: IntakeSession): StoredIntakeSession {
  return {
    id: session.id,
    question_set_version: session.question_set_version,
    status: session.status,
    missing_fields: session.missing_fields,
    phase: session.phase,
    completed_phase_one_fields: session.completed_phase_one_fields,
    missing_phase_one_fields: session.missing_phase_one_fields,
    phase_transition_ready: session.phase_transition_ready,
    next_question: session.next_question,
    guidance_mode: session.guidance_mode,
    ai_initial_review_status: session.ai_initial_review_status,
    privacy_notice: session.privacy_notice,
  };
}

function readStoredState(storageKey: string): StoredIntakeState | null {
  try {
    const value = window.sessionStorage.getItem(storageKey);
    if (!value) return null;
    const parsed = JSON.parse(value) as Partial<StoredIntakeState>;
    const session = parsed.session;
    if (!session || typeof session.id !== "string")
      return discardStoredState(storageKey);
    if (
      session.question_set_version !== undefined &&
      (!Number.isInteger(session.question_set_version) ||
        session.question_set_version < 1)
    )
      return discardStoredState(storageKey);
    if (
      ![
        "collecting",
        "ready_for_confirmation",
        "awaiting_family_review",
        "ready_for_second_confirmation",
      ].includes(session.status)
    )
      return discardStoredState(storageKey);
    if (!["phase_one", "phase_two"].includes(session.phase))
      return discardStoredState(storageKey);
    if (!Array.isArray(session.missing_fields))
      return discardStoredState(storageKey);
    if (!Array.isArray(session.completed_phase_one_fields))
      return discardStoredState(storageKey);
    if (!Array.isArray(session.missing_phase_one_fields))
      return discardStoredState(storageKey);
    if (typeof session.phase_transition_ready !== "boolean")
      return discardStoredState(storageKey);
    if (
      session.next_question !== null &&
      (typeof session.next_question !== "object" ||
        typeof session.next_question.field !== "string" ||
        typeof session.next_question.prompt !== "string" ||
        typeof session.next_question.required !== "boolean")
    )
      return discardStoredState(storageKey);
    if (
      !session.next_question &&
      ![
        "ready_for_confirmation",
        "awaiting_family_review",
        "ready_for_second_confirmation",
      ].includes(session.status)
    )
      return discardStoredState(storageKey);
    if (typeof session.ai_initial_review_status !== "string")
      return discardStoredState(storageKey);
    return {
      session: {
        ...session,
        question_set_version: session.question_set_version ?? 2,
      } as StoredIntakeSession,
      answer: typeof parsed.answer === "string" ? parsed.answer : "",
      basicInformation: readBasicInformation(parsed.basicInformation),
      aiExecution: readAiExecution(parsed.aiExecution),
    };
  } catch {
    return discardStoredState(storageKey);
  }
}

function readAiExecution(
  value: unknown,
): { id: string; workflow: string } | undefined {
  if (!value || typeof value !== "object") return undefined;
  const execution = value as Partial<{ id: string; workflow: string }>;
  if (typeof execution.id !== "string" || !execution.id) return undefined;
  if (typeof execution.workflow !== "string" || !execution.workflow)
    return undefined;
  return { id: execution.id, workflow: execution.workflow };
}

function readBasicInformation(value: unknown): BasicInformationDraft {
  if (!value || typeof value !== "object") return blankBasicInformation;
  const draft = value as Partial<BasicInformationDraft>;
  return {
    name: typeof draft.name === "string" ? draft.name : "",
    gender: typeof draft.gender === "string" ? draft.gender : "",
    age: typeof draft.age === "string" ? draft.age : "",
    height: typeof draft.height === "string" ? draft.height : "",
    appearance: typeof draft.appearance === "string" ? draft.appearance : "",
  };
}

function discardStoredState(storageKey: string): null {
  window.sessionStorage.removeItem(storageKey);
  return null;
}

function clearStoredState(storageKey: string) {
  if (typeof window !== "undefined")
    window.sessionStorage.removeItem(storageKey);
}

function questionLabel(field: string) {
  return questionLabels[field] ?? field;
}

function assessmentLabel(severity: IntakeAssessment["severity"]) {
  return severity === "blocking"
    ? "需要先处理"
    : severity === "warning"
      ? "请注意"
      : "提示";
}

function assessmentClass(severity: IntakeAssessment["severity"]) {
  return severity === "blocking"
    ? "border-red-200 bg-red-50 text-red-800"
    : severity === "warning"
      ? "border-amber-200 bg-amber-50 text-amber-900"
      : "border-blue-200 bg-blue-50 text-blue-900";
}

function nullable(value: string): string | null {
  const trimmed = value.trim();
  return trimmed || null;
}

function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(date);
}

function toDateTimeLocal(value: string | null) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

function messageFrom(cause: unknown) {
  return cause instanceof ApiClientError
    ? cause.message
    : cause instanceof Error
      ? cause.message
      : "操作失败，请稍后重试。";
}
