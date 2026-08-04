import { Button, Input, Spinner } from "@heroui/react";
import { BookCheck, FilePlus2, ShieldCheck } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  createManagedLearningQuestion,
  createManagedLearningResource,
  listManagedLearningQuestions,
  listManagedLearningResources,
  transitionManagedLearningQuestion,
  transitionManagedLearningResource,
} from "../api/learning";
import type {
  CreateLearningQuestionInput,
  CreateLearningResourceInput,
  LearningContentLifecycle,
  ManagedLearningQuestion,
  ManagedLearningResource,
} from "../api/learning";
import { ApiClientError } from "../api/client";
import { useAuth } from "../auth/useAuth";
import { ErrorState, LoadingState } from "../components/ContentState";

const effectiveAt = () => new Date().toISOString();

function TextareaField({ label, value, onChange, className = "", minRows = 3, required = false }: { label: string; value: string; onChange: (event: React.ChangeEvent<HTMLTextAreaElement>) => void; className?: string; minRows?: number; required?: boolean }) {
  return <label className={`grid gap-1 text-sm text-slate-700 ${className}`}><span>{label}{required && <span className="ml-1 text-rose-600">*</span>}</span><textarea className="min-h-24 rounded-md border border-slate-300 bg-white px-3 py-2 text-sm outline-none focus:border-brand-600 focus:ring-2 focus:ring-brand-100" rows={minRows} value={value} onChange={onChange} required={required} /></label>;
}

function errorMessage(cause: unknown) {
  return cause instanceof ApiClientError
    ? cause.detail ?? cause.message
    : "暂时无法完成内容治理操作，请稍后重试。";
}

function parseTags(value: string) {
  return value.split("，").flatMap((part) => part.split(",")).map((tag) => tag.trim()).filter(Boolean);
}

function transitionLabel(state: LearningContentLifecycle["state"]) {
  const labels: Record<LearningContentLifecycle["state"], string> = {
    submitted: "待脱敏",
    deidentified: "待审核",
    reviewed: "待发布",
    published: "已发布",
    withdrawn: "已撤回",
    unmanaged: "未纳入治理",
  };
  return labels[state];
}

function LifecycleActions({
  lifecycle,
  currentUserId,
  onAction,
  busy,
}: {
  lifecycle: LearningContentLifecycle;
  currentUserId?: string;
  onAction: (action: "deidentify" | "review" | "publish" | "withdraw") => void;
  busy: boolean;
}) {
  const action = lifecycle.state === "submitted" ? "deidentify" : lifecycle.state === "deidentified" ? "review" : lifecycle.state === "reviewed" ? "publish" : lifecycle.state === "published" ? "withdraw" : null;
  const labels = { deidentify: "确认脱敏", review: "确认审核", publish: "发布", withdraw: "撤回" };
  const isSubmitter = lifecycle.submitted_by_user_id === currentUserId;
  const requiresIndependentReviewer = action === "deidentify" || action === "review";
  const disabledForIndependence = requiresIndependentReviewer && isSubmitter;
  const description = disabledForIndependence
    ? action === "deidentify"
      ? "提交人不能确认本条内容的脱敏，请由另一名管理员处理。"
      : "提交人不能确认本条内容的审核，请由另一名管理员处理。"
    : null;

  return action ? <div className="grid justify-items-end gap-1"><Button size="sm" variant={action === "withdraw" ? "danger" : "secondary"} isDisabled={busy || disabledForIndependence} onPress={() => onAction(action)}>{labels[action]}</Button>{description && <span className="max-w-64 text-right text-xs leading-5 text-amber-700">{description}</span>}</div> : null;
}

export function LearningGovernancePage() {
  const { token, user } = useAuth();
  const [resources, setResources] = useState<ManagedLearningResource[]>([]);
  const [questions, setQuestions] = useState<ManagedLearningQuestion[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [operationError, setOperationError] = useState("");
  const [busyId, setBusyId] = useState("");
  const [reason, setReason] = useState("");
  const [resourceForm, setResourceForm] = useState<CreateLearningResourceInput>({ title: "", summary: "", content: "", resource_type: "manual", tags: [], source_name: "", source_url: null, visibility: "learner", effective_at: effectiveAt(), permitted_use: "training", submission_reason: "" });
  const [resourceTags, setResourceTags] = useState("");
  const [questionForm, setQuestionForm] = useState<CreateLearningQuestionInput>({ source_resource_id: "", prompt: "", question_type: "single_choice", difficulty: "basic", tags: [], options: [], correct_option_id: "", explanation: "", visibility: "learner", effective_at: effectiveAt(), permitted_use: "training", submission_reason: "" });
  const [questionTags, setQuestionTags] = useState("");
  const [optionLines, setOptionLines] = useState("");

  const load = useCallback(async () => {
    if (!token) return;
    setIsLoading(true);
    setLoadError("");
    try {
      const [nextResources, nextQuestions] = await Promise.all([listManagedLearningResources(token), listManagedLearningQuestions(token)]);
      setResources(nextResources);
      setQuestions(nextQuestions);
    } catch (cause) {
      setLoadError(errorMessage(cause));
    } finally {
      setIsLoading(false);
    }
  }, [token]);

  useEffect(() => { void load(); }, [load]);
  if (isLoading) return <LoadingState label="正在加载学习内容治理记录" />;
  if (loadError) return <ErrorState message={loadError} onRetry={() => void load()} />;

  const actOnResource = (resource: ManagedLearningResource, action: "deidentify" | "review" | "publish" | "withdraw") => {
    if (!token || !reason.trim()) { setOperationError("请填写本次操作理由。"); return; }
    setOperationError("");
    setBusyId(resource.id);
    void transitionManagedLearningResource(token, resource.id, action, reason).then(() => { setReason(""); return load(); }).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId(""));
  };
  const actOnQuestion = (question: ManagedLearningQuestion, action: "deidentify" | "review" | "publish" | "withdraw") => {
    if (!token || !reason.trim()) { setOperationError("请填写本次操作理由。"); return; }
    setOperationError("");
    setBusyId(question.id);
    void transitionManagedLearningQuestion(token, question.id, action, reason).then(() => { setReason(""); return load(); }).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId(""));
  };
  const submitResource = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token) return;
    setOperationError("");
    setBusyId("new-resource");
    void createManagedLearningResource(token, { ...resourceForm, tags: parseTags(resourceTags), source_url: resourceForm.source_url?.trim() || null }).then(() => { setResourceForm({ ...resourceForm, title: "", summary: "", content: "", tags: [], source_name: "", source_url: null, submission_reason: "" }); setResourceTags(""); return load(); }).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId(""));
  };
  const submitQuestion = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token) return;
    setOperationError("");
    const options = optionLines.split("\n").map((line) => line.split("|")).filter(([id, text]) => id?.trim() && text?.trim()).map(([id, text]) => ({ id: id.trim(), text: text.trim() }));
    setBusyId("new-question");
    void createManagedLearningQuestion(token, { ...questionForm, tags: parseTags(questionTags), options }).then(() => { setQuestionForm({ ...questionForm, prompt: "", tags: [], options: [], correct_option_id: "", explanation: "", submission_reason: "" }); setQuestionTags(""); setOptionLines(""); return load(); }).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId(""));
  };

  return <main className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
    <header className="mb-7 flex items-start gap-3"><span className="grid size-11 place-items-center rounded-md bg-emerald-50 text-emerald-700"><ShieldCheck aria-hidden="true" /></span><div><h1 className="m-0 text-2xl font-bold text-slate-950">学习内容治理</h1><p className="mb-0 mt-1 text-sm text-slate-600">仅管理员可操作</p></div></header>
    <div className="mb-5"><Input aria-label="操作理由" value={reason} onChange={(event) => setReason(event.target.value)} maxLength={1000} placeholder="操作理由" /></div>
    {operationError && <div className="mb-5 border border-rose-200 bg-rose-50 px-4 py-3 text-sm leading-6 text-rose-800" role="alert">{operationError}</div>}
    <section className="border-y border-slate-200 bg-white"><header className="flex items-center gap-2 border-b border-slate-200 px-5 py-4"><FilePlus2 size={18} aria-hidden="true" /><h2 className="m-0 text-base font-bold text-slate-950">新增学习资源</h2></header><form className="grid gap-4 p-5 lg:grid-cols-2" onSubmit={submitResource}>
      <label className="grid gap-1 text-sm text-slate-700">标题<Input aria-label="标题" value={resourceForm.title} onChange={(event) => setResourceForm({ ...resourceForm, title: event.target.value })} required /></label><label className="grid gap-1 text-sm text-slate-700">来源名称<Input aria-label="来源名称" value={resourceForm.source_name} onChange={(event) => setResourceForm({ ...resourceForm, source_name: event.target.value })} required /></label>
      <label className="grid gap-1 text-sm text-slate-700">资源类型<select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={resourceForm.resource_type} onChange={(event) => setResourceForm({ ...resourceForm, resource_type: event.target.value as CreateLearningResourceInput["resource_type"] })}><option value="manual">手册</option><option value="prevention">防走失知识</option><option value="team_intro">队伍介绍</option><option value="case_study">案例学习</option></select></label><label className="grid gap-1 text-sm text-slate-700">可见范围<select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={resourceForm.visibility} onChange={(event) => setResourceForm({ ...resourceForm, visibility: event.target.value as CreateLearningResourceInput["visibility"] })}><option value="learner">新人</option><option value="volunteer">志愿者</option><option value="authenticated">登录用户</option><option value="public">公开</option></select></label>
      <label className="grid gap-1 text-sm text-slate-700">来源链接（HTTPS）<Input aria-label="来源链接（HTTPS）" value={resourceForm.source_url ?? ""} onChange={(event) => setResourceForm({ ...resourceForm, source_url: event.target.value })} /></label><label className="grid gap-1 text-sm text-slate-700">标签（逗号分隔）<Input aria-label="标签（逗号分隔）" value={resourceTags} onChange={(event) => setResourceTags(event.target.value)} /></label>
      <TextareaField className="lg:col-span-2" label="摘要" value={resourceForm.summary} onChange={(event) => setResourceForm({ ...resourceForm, summary: event.target.value })} required /><TextareaField className="lg:col-span-2" label="正文" value={resourceForm.content} onChange={(event) => setResourceForm({ ...resourceForm, content: event.target.value })} minRows={5} required />
      <TextareaField className="lg:col-span-2" label="提交理由" value={resourceForm.submission_reason} onChange={(event) => setResourceForm({ ...resourceForm, submission_reason: event.target.value })} required /><div className="lg:col-span-2"><Button type="submit" isDisabled={busyId === "new-resource"}>{busyId === "new-resource" ? <Spinner size="sm" /> : "提交资源"}</Button></div>
    </form></section>
    <section className="mt-7 border-y border-slate-200 bg-white"><header className="border-b border-slate-200 px-5 py-4"><h2 className="m-0 text-base font-bold text-slate-950">资源治理记录</h2></header><div className="divide-y divide-slate-100">{resources.length === 0 ? <p className="m-0 px-5 py-8 text-sm text-slate-600">暂无资源记录</p> : resources.map((resource) => <article key={resource.id} className="flex flex-wrap items-center justify-between gap-3 px-5 py-4"><div><h3 className="m-0 text-sm font-semibold text-slate-950">{resource.title}</h3><p className="mb-0 mt-1 text-xs text-slate-500">v{resource.version} · {transitionLabel(resource.lifecycle.state)} · {resource.source_name}</p></div><LifecycleActions lifecycle={resource.lifecycle} currentUserId={user?.id} busy={busyId === resource.id} onAction={(action) => actOnResource(resource, action)} /></article>)}</div></section>
    <section className="mt-7 border-y border-slate-200 bg-white"><header className="flex items-center gap-2 border-b border-slate-200 px-5 py-4"><BookCheck size={18} aria-hidden="true" /><h2 className="m-0 text-base font-bold text-slate-950">新增学习题目</h2></header><form className="grid gap-4 p-5 lg:grid-cols-2" onSubmit={submitQuestion}>
      <label className="grid gap-1 text-sm text-slate-700">来源资源<select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={questionForm.source_resource_id} onChange={(event) => setQuestionForm({ ...questionForm, source_resource_id: event.target.value })} required><option value="">选择资源</option>{resources.filter((resource) => resource.lifecycle.state === "published").map((resource) => <option key={resource.id} value={resource.id}>{resource.title}</option>)}</select></label><label className="grid gap-1 text-sm text-slate-700">正确选项编号<Input aria-label="正确选项编号" value={questionForm.correct_option_id} onChange={(event) => setQuestionForm({ ...questionForm, correct_option_id: event.target.value })} required /></label>
      <label className="grid gap-1 text-sm text-slate-700">题目类型<select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={questionForm.question_type} onChange={(event) => setQuestionForm({ ...questionForm, question_type: event.target.value as CreateLearningQuestionInput["question_type"] })}><option value="single_choice">选择题</option><option value="true_false">判断题</option><option value="scenario">情景题</option></select></label><label className="grid gap-1 text-sm text-slate-700">难度<select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={questionForm.difficulty} onChange={(event) => setQuestionForm({ ...questionForm, difficulty: event.target.value as CreateLearningQuestionInput["difficulty"] })}><option value="basic">基础</option><option value="intermediate">进阶</option><option value="advanced">高级</option></select></label>
      <TextareaField className="lg:col-span-2" label="题干" value={questionForm.prompt} onChange={(event) => setQuestionForm({ ...questionForm, prompt: event.target.value })} required /><TextareaField className="lg:col-span-2" label="选项（每行：编号|文本）" value={optionLines} onChange={(event) => setOptionLines(event.target.value)} minRows={3} required />
      <label className="grid gap-1 text-sm text-slate-700">标签（逗号分隔）<Input aria-label="标签（逗号分隔）" value={questionTags} onChange={(event) => setQuestionTags(event.target.value)} /></label><TextareaField label="解析" value={questionForm.explanation} onChange={(event) => setQuestionForm({ ...questionForm, explanation: event.target.value })} required />
      <TextareaField className="lg:col-span-2" label="提交理由" value={questionForm.submission_reason} onChange={(event) => setQuestionForm({ ...questionForm, submission_reason: event.target.value })} required /><div className="lg:col-span-2"><Button type="submit" isDisabled={busyId === "new-question"}>{busyId === "new-question" ? <Spinner size="sm" /> : "提交题目"}</Button></div>
    </form></section>
    <section className="mt-7 border-y border-slate-200 bg-white"><header className="border-b border-slate-200 px-5 py-4"><h2 className="m-0 text-base font-bold text-slate-950">题目治理记录</h2></header><div className="divide-y divide-slate-100">{questions.length === 0 ? <p className="m-0 px-5 py-8 text-sm text-slate-600">暂无题目记录</p> : questions.map((question) => <article key={question.id} className="flex flex-wrap items-center justify-between gap-3 px-5 py-4"><div><h3 className="m-0 text-sm font-semibold text-slate-950">{question.prompt}</h3><p className="mb-0 mt-1 text-xs text-slate-500">v{question.version} · {transitionLabel(question.lifecycle.state)}</p></div><LifecycleActions lifecycle={question.lifecycle} currentUserId={user?.id} busy={busyId === question.id} onAction={(action) => actOnQuestion(question, action)} /></article>)}</div></section>
  </main>;
}
