import { Button, Input, Spinner } from "@heroui/react";
import { BookCheck, FilePlus2, ShieldCheck } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  createManagedLearningQuestion,
  createManagedLearningResource,
  listManagedLearningCategories,
  listManagedLearningQuestions,
  listManagedLearningResources,
  transitionManagedLearningCategory,
  transitionManagedLearningQuestion,
  transitionManagedLearningResource,
  createKnowledgeBase,
  listKnowledgeBases,
  previewKnowledgeImport,
  confirmKnowledgeImport,
  cancelKnowledgeImport,
  createKnowledgeItem,
  listKnowledgeItems,
  transitionKnowledgeItem,
  getKnowledgeOverview,
  uploadKnowledgeImage,
} from "../api/learning";
import type {
  CreateLearningQuestionInput,
  CreateLearningResourceInput,
  LearningContentLifecycle,
  ManagedLearningCategory,
  ManagedLearningQuestion,
  ManagedLearningResource,
} from "../api/learning";
import { ApiClientError } from "../api/client";
import { useAuth } from "../auth/useAuth";
import { ErrorState, LoadingState } from "../components/ContentState";

const effectiveAt = () => new Date().toISOString();

function TextareaField({
  label,
  value,
  onChange,
  className = "",
  minRows = 3,
  required = false,
}: {
  label: string;
  value: string;
  onChange: (event: React.ChangeEvent<HTMLTextAreaElement>) => void;
  className?: string;
  minRows?: number;
  required?: boolean;
}) {
  return (
    <label className={`grid gap-1 text-sm text-slate-700 ${className}`}>
      <span>
        {label}
        {required && <span className="ml-1 text-rose-600">*</span>}
      </span>
      <textarea
        className="min-h-24 rounded-md border border-slate-300 bg-white px-3 py-2 text-sm outline-none focus:border-brand-600 focus:ring-2 focus:ring-brand-100"
        rows={minRows}
        value={value}
        onChange={onChange}
        required={required}
      />
    </label>
  );
}

function errorMessage(cause: unknown) {
  return cause instanceof ApiClientError
    ? (cause.detail ?? cause.message)
    : "暂时无法完成内容治理操作，请稍后重试。";
}

function isMissingCategoryEndpoint(cause: unknown) {
  return (
    cause instanceof ApiClientError &&
    cause.status === 404 &&
    cause.code === "not_found"
  );
}

function parseTags(value: string) {
  return value
    .split("，")
    .flatMap((part) => part.split(","))
    .map((tag) => tag.trim())
    .filter(Boolean);
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
  canAbandonCorrection,
}: {
  lifecycle: LearningContentLifecycle;
  currentUserId?: string;
  onAction: (action: "deidentify" | "review" | "publish" | "withdraw") => void;
  busy: boolean;
  canAbandonCorrection: boolean;
}) {
  const action =
    lifecycle.state === "submitted"
      ? "deidentify"
      : lifecycle.state === "deidentified"
        ? "review"
        : lifecycle.state === "reviewed"
          ? "publish"
          : lifecycle.state === "published"
            ? "withdraw"
            : null;
  const labels = {
    deidentify: "确认脱敏",
    review: "确认审核",
    publish: "发布",
    withdraw: "撤回",
  };
  const isSubmitter = lifecycle.submitted_by_user_id === currentUserId;
  const requiresIndependentReviewer =
    action === "deidentify" || action === "review";
  const disabledForIndependence = requiresIndependentReviewer && isSubmitter;
  const description = disabledForIndependence
    ? action === "deidentify"
      ? "提交人不能确认本条内容的脱敏，请由另一名管理员处理。"
      : "提交人不能确认本条内容的审核，请由另一名管理员处理。"
    : null;

  const canAbandon =
    canAbandonCorrection &&
    ["submitted", "deidentified", "reviewed"].includes(lifecycle.state);
  return action || canAbandon ? (
    <div className="grid justify-items-end gap-1">
      {action && (
        <Button
          size="sm"
          variant={action === "withdraw" ? "danger" : "secondary"}
          isDisabled={busy || disabledForIndependence}
          onPress={() => onAction(action)}
        >
          {labels[action]}
        </Button>
      )}
      {canAbandon && (
        <Button
          size="sm"
          variant="danger"
          isDisabled={busy}
          onPress={() => onAction("withdraw")}
        >
          放弃更正
        </Button>
      )}
      {description && (
        <span className="max-w-64 text-right text-xs leading-5 text-amber-700">
          {description}
        </span>
      )}
    </div>
  ) : null;
}

export function LearningGovernancePage() {
  const { token, user } = useAuth();
  const [resources, setResources] = useState<ManagedLearningResource[]>([]);
  const [categories, setCategories] = useState<ManagedLearningCategory[]>([]);
  const [questions, setQuestions] = useState<ManagedLearningQuestion[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const [operationError, setOperationError] = useState("");
  const [busyId, setBusyId] = useState("");
  const [reason, setReason] = useState("");
  const [resourceForm, setResourceForm] = useState<CreateLearningResourceInput>(
    {
      title: "",
      summary: "",
      content: "",
      resource_type: "manual",
      tags: [],
      category_id: null,
      source_name: "",
      source_url: null,
      visibility: "learner",
      effective_at: effectiveAt(),
      permitted_use: "training",
      submission_reason: "",
    },
  );
  const [resourceTags, setResourceTags] = useState("");
  const [questionForm, setQuestionForm] = useState<CreateLearningQuestionInput>(
    {
      source_resource_id: "",
      prompt: "",
      question_type: "single_choice",
      difficulty: "basic",
      tags: [],
      options: [],
      correct_option_id: "",
      explanation: "",
      visibility: "learner",
      effective_at: effectiveAt(),
      permitted_use: "training",
      submission_reason: "",
    },
  );
  const [questionTags, setQuestionTags] = useState("");
  const [optionLines, setOptionLines] = useState("");
  const [knowledgeBases, setKnowledgeBases] = useState<import("../api/learning").KnowledgeBase[]>([]);
  const [knowledgeOverview, setKnowledgeOverview] = useState<import("../api/learning").KnowledgeOverview | null>(null);
  const [knowledgeBaseName, setKnowledgeBaseName] = useState("");
  const [knowledgeBaseDescription, setKnowledgeBaseDescription] = useState("");
  const [knowledgeImport, setKnowledgeImport] = useState<import("../api/learning").KnowledgeImportBatch | null>(null);
  const [knowledgeItems, setKnowledgeItems] = useState<import("../api/learning").KnowledgeItem[]>([]);
  const [knowledgeItemBaseId, setKnowledgeItemBaseId] = useState("");
  const [knowledgeItemForm, setKnowledgeItemForm] = useState({ title: "", summary: "", content: "", category: "", keywords: "", source_name: "", source_url: "" });

  const load = useCallback(async () => {
    if (!token) return;
    setIsLoading(true);
    setLoadError("");
    try {
      const [nextResources, nextQuestions] = await Promise.all([
        listManagedLearningResources(token),
        listManagedLearningQuestions(token),
      ]);
      setResources(nextResources);
      setQuestions(nextQuestions);
      const bases = await listKnowledgeBases(token).catch(() => []);
      setKnowledgeBases(bases);
      if (bases[0]) {
        setKnowledgeOverview(
          await getKnowledgeOverview(token, bases[0].id).catch(() => null),
        );
      } else {
        setKnowledgeOverview(null);
      }
      // Keep resource governance usable while rolling out the optional
      // category-governance endpoint to older deployments.
      const nextCategories =
        typeof listManagedLearningCategories === "function"
          ? await listManagedLearningCategories(token).catch((cause) => {
              if (isMissingCategoryEndpoint(cause)) return [];
              throw cause;
            })
          : [];
      setCategories(nextCategories);
    } catch (cause) {
      setLoadError(errorMessage(cause));
    } finally {
      setIsLoading(false);
    }
  }, [token]);

  useEffect(() => {
    void load();
  }, [load]);

  const createBase = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token || !knowledgeBaseName.trim()) return;
    setBusyId("knowledge-base");
    void createKnowledgeBase(token, { name: knowledgeBaseName, description: knowledgeBaseDescription, visibility: "learner" })
      .then((base) => { setKnowledgeBases((current) => [...current, base]); setKnowledgeBaseName(""); setKnowledgeBaseDescription(""); })
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };
  const uploadKnowledgeCsv = (baseId: string, file: File) => {
    if (!token) return;
    setBusyId("knowledge-import");
    void previewKnowledgeImport(token, baseId, file)
      .then(setKnowledgeImport)
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };
  const updateImport = (action: "confirm" | "cancel") => {
    if (!token || !knowledgeImport) return;
    setBusyId("knowledge-import");
    const operation = action === "confirm" ? confirmKnowledgeImport : cancelKnowledgeImport;
    void operation(token, knowledgeImport.id).then(setKnowledgeImport).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId(""));
  };
  const loadKnowledgeItems = (baseId: string) => { if (!token || !baseId) return; setKnowledgeItemBaseId(baseId); void Promise.all([listKnowledgeItems(token, baseId), getKnowledgeOverview(token, baseId)]).then(([items, overview]) => { setKnowledgeItems(items); setKnowledgeOverview(overview); }).catch((cause) => setOperationError(errorMessage(cause))); };
  const uploadItemImages = (itemId: string, files: FileList | null) => { if (!token || !files?.length) return; setBusyId(itemId); void Promise.all(Array.from(files).map((file) => uploadKnowledgeImage(token, itemId, file))).then(() => loadKnowledgeItems(knowledgeItemBaseId)).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId("")); };
  const createItem = (event: React.FormEvent<HTMLFormElement>) => { event.preventDefault(); if (!token || !knowledgeItemBaseId) return; setBusyId("knowledge-item"); const images: { storage_path: string; mime_type: string; width: number | null; height: number | null; metadata: Record<string, unknown> }[] = []; void createKnowledgeItem(token, knowledgeItemBaseId, { title: knowledgeItemForm.title, summary: knowledgeItemForm.summary, content: knowledgeItemForm.content, category: knowledgeItemForm.category, category_id: null, keywords: parseTags(knowledgeItemForm.keywords), source_name: knowledgeItemForm.source_name, source_url: knowledgeItemForm.source_url.trim() || null, visibility: "learner", images }).then(() => { setKnowledgeItemForm({ title: "", summary: "", content: "", category: "", keywords: "", source_name: "", source_url: "" }); return listKnowledgeItems(token, knowledgeItemBaseId); }).then(setKnowledgeItems).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId("")); };
  const transitionItem = (itemId: string, action: "review" | "publish" | "withdraw") => { if (!token) return; setBusyId(itemId); void transitionKnowledgeItem(token, itemId, action).then(() => listKnowledgeItems(token, knowledgeItemBaseId)).then(setKnowledgeItems).catch((cause) => setOperationError(errorMessage(cause))).finally(() => setBusyId("")); };
  if (isLoading) return <LoadingState label="正在加载学习内容治理记录" />;
  if (loadError)
    return <ErrorState message={loadError} onRetry={() => void load()} />;

  const actOnResource = (
    resource: ManagedLearningResource,
    action: "deidentify" | "review" | "publish" | "withdraw",
  ) => {
    if (!token || !reason.trim()) {
      setOperationError("请填写本次操作理由。");
      return;
    }
    setOperationError("");
    setBusyId(resource.id);
    void transitionManagedLearningResource(token, resource.id, action, reason)
      .then(() => {
        setReason("");
        return load();
      })
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };
  const actOnQuestion = (
    question: ManagedLearningQuestion,
    action: "deidentify" | "review" | "publish" | "withdraw",
  ) => {
    if (!token || !reason.trim()) {
      setOperationError("请填写本次操作理由。");
      return;
    }
    setOperationError("");
    setBusyId(question.id);
    void transitionManagedLearningQuestion(token, question.id, action, reason)
      .then(() => {
        setReason("");
        return load();
      })
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };
  const actOnCategory = (
    category: ManagedLearningCategory,
    action: "enable" | "reject" | "disable",
  ) => {
    if (!token || !reason.trim()) {
      setOperationError("请填写本次操作理由。");
      return;
    }
    setOperationError("");
    setBusyId(category.id);
    void transitionManagedLearningCategory(token, category.id, action, reason)
      .then(() => {
        setReason("");
        return load();
      })
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };

  const submitResource = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token) return;
    setOperationError("");
    setBusyId("new-resource");
    void createManagedLearningResource(token, {
      ...resourceForm,
      tags: parseTags(resourceTags),
      source_url: resourceForm.source_url?.trim() || null,
    })
      .then(() => {
        setResourceForm({
          ...resourceForm,
          title: "",
          summary: "",
          content: "",
          tags: [],
          category_id: null,
          source_name: "",
          source_url: null,
          previous_version_id: null,
          submission_reason: "",
        });
        setResourceTags("");
        return load();
      })
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };
  const submitQuestion = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token) return;
    setOperationError("");
    const options = optionLines
      .split("\n")
      .map((line) => line.split("|"))
      .filter(([id, text]) => id?.trim() && text?.trim())
      .map(([id, text]) => ({ id: id.trim(), text: text.trim() }));
    setBusyId("new-question");
    void createManagedLearningQuestion(token, {
      ...questionForm,
      tags: parseTags(questionTags),
      options,
    })
      .then(() => {
        setQuestionForm({
          ...questionForm,
          prompt: "",
          tags: [],
          options: [],
          correct_option_id: "",
          explanation: "",
          previous_version_id: null,
          submission_reason: "",
        });
        setQuestionTags("");
        setOptionLines("");
        return load();
      })
      .catch((cause) => setOperationError(errorMessage(cause)))
      .finally(() => setBusyId(""));
  };
  const correctionSourceResourceId = questionForm.previous_version_id
    ? questions.find((question) => question.id === questionForm.previous_version_id)
        ?.source_resource_id
    : null;

  return (
    <main className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 flex items-start gap-3">
        <span className="grid size-11 place-items-center rounded-md bg-emerald-50 text-emerald-700">
          <ShieldCheck aria-hidden="true" />
        </span>
        <div>
          <h1 className="m-0 text-2xl font-bold text-slate-950">
            学习内容治理
          </h1>
          <p className="mb-0 mt-1 text-sm text-slate-600">仅管理员可操作</p>
        </div>
      </header>
      <section className="mb-7 border-y border-slate-200 bg-white" aria-labelledby="knowledge-library-title">
        <header className="border-b border-slate-200 px-5 py-4">
          <h2 id="knowledge-library-title" className="m-0 text-base font-bold text-slate-950">资料库管理</h2>
        </header>
        {knowledgeOverview && <div className="grid grid-cols-2 gap-px border-b border-slate-200 bg-slate-200 sm:grid-cols-4">
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Knowledge bases</span><strong>{knowledgeOverview.total_bases}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Enabled</span><strong>{knowledgeOverview.enabled_bases}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Items</span><strong>{knowledgeOverview.total_items}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Published</span><strong>{knowledgeOverview.published_items}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Waiting review</span><strong>{knowledgeOverview.draft_items}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Reviewed</span><strong>{knowledgeOverview.reviewed_items}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Withdrawn</span><strong>{knowledgeOverview.withdrawn_items}</strong></div>
          <div className="bg-white px-5 py-3 text-sm"><span className="block text-xs text-slate-500">Images</span><strong>{knowledgeOverview.image_count}</strong></div>
        </div>}
        <form className="grid gap-3 border-b border-slate-100 p-5 lg:grid-cols-3" onSubmit={createBase}>
          <label className="grid gap-1 text-sm text-slate-700">资料库名称<Input aria-label="资料库名称" value={knowledgeBaseName} onChange={(event) => setKnowledgeBaseName(event.target.value)} required /></label>
          <label className="grid gap-1 text-sm text-slate-700 lg:col-span-2">说明<Input aria-label="说明" value={knowledgeBaseDescription} onChange={(event) => setKnowledgeBaseDescription(event.target.value)} /></label>
          <div className="lg:col-span-3"><Button type="submit" isDisabled={busyId === "knowledge-base"}>{busyId === "knowledge-base" ? <Spinner size="sm" /> : "新建资料库"}</Button></div>
        </form>
        <div className="divide-y divide-slate-100">
          {knowledgeBases.map((base) => <article key={base.id} className="grid gap-3 px-5 py-4 lg:grid-cols-[1fr_auto]">
            <div><h3 className="m-0 text-sm font-semibold text-slate-950">{base.name}</h3><p className="mb-0 mt-1 text-xs text-slate-500">{base.description || "无说明"} · {base.status}</p></div>
            <div className="flex flex-wrap items-center gap-3"><Button variant="secondary" onPress={() => loadKnowledgeItems(base.id)}>查看条目</Button><label className="text-sm text-slate-700">CSV 导入<input className="ml-2 text-xs" type="file" accept=".csv,text/csv" disabled={busyId === "knowledge-import"} onChange={(event) => { const file = event.target.files?.[0]; event.currentTarget.value = ""; if (file) uploadKnowledgeCsv(base.id, file); }} /></label></div>
          </article>)}
        </div>
        {knowledgeItemBaseId && <div className="border-t border-slate-200 p-5">
          <h3 className="m-0 text-sm font-semibold text-slate-950">手工录入知识条目</h3>
          <form className="mt-3 grid gap-3 lg:grid-cols-2" onSubmit={createItem}>
            <Input aria-label="知识条目标题" placeholder="标题" value={knowledgeItemForm.title} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, title: event.target.value })} required />
            <Input aria-label="来源名称" placeholder="来源名称" value={knowledgeItemForm.source_name} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, source_name: event.target.value })} required />
            <Input aria-label="分类" placeholder="分类" value={knowledgeItemForm.category} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, category: event.target.value })} />
            <Input aria-label="关键词" placeholder="关键词，逗号分隔" value={knowledgeItemForm.keywords} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, keywords: event.target.value })} />
            <Input className="lg:col-span-2" aria-label="摘要" placeholder="摘要" value={knowledgeItemForm.summary} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, summary: event.target.value })} />
            <textarea aria-label="正文" className="min-h-28 rounded-md border border-slate-300 px-3 py-2 text-sm lg:col-span-2" placeholder="正文" value={knowledgeItemForm.content} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, content: event.target.value })} required />
            <Input aria-label="来源链接" placeholder="HTTPS 来源链接" value={knowledgeItemForm.source_url} onChange={(event) => setKnowledgeItemForm({ ...knowledgeItemForm, source_url: event.target.value })} />
            <div className="lg:col-span-2"><Button type="submit" isDisabled={busyId === "knowledge-item"}>{busyId === "knowledge-item" ? <Spinner size="sm" /> : "保存知识条目"}</Button></div>
          </form>
          <div className="mt-5 divide-y divide-slate-100 border-t border-slate-100">
            {knowledgeItems.map((item) => (
              <article key={item.knowledge_item_id} className="py-3">
                <div className="flex flex-wrap items-center gap-2">
                  <strong className="text-sm">{item.title}</strong>
                  <span className="text-xs text-slate-500">v{item.version} · {item.status}</span>
                  {item.images.length > 0 && <span className="text-xs text-slate-500">{item.images.length} images</span>}
                  {(item.status === "draft" || item.status === "submitted") && <Button size="sm" variant="secondary" isDisabled={busyId === item.knowledge_item_id} onPress={() => transitionItem(item.knowledge_item_id, "review")}>Review</Button>}
                  {item.status === "reviewed" && <Button size="sm" isDisabled={busyId === item.knowledge_item_id} onPress={() => transitionItem(item.knowledge_item_id, "publish")}>Publish</Button>}
                  {item.status === "published" && <Button size="sm" variant="secondary" isDisabled={busyId === item.knowledge_item_id} onPress={() => transitionItem(item.knowledge_item_id, "withdraw")}>Withdraw</Button>}
                  <label className="text-xs text-slate-700">Upload images<input className="ml-2 text-xs" type="file" accept="image/jpeg,image/png" multiple disabled={busyId === item.knowledge_item_id} onChange={(event) => { uploadItemImages(item.knowledge_item_id, event.target.files); event.currentTarget.value = ""; }} /></label>
                </div>
              </article>
            ))}
          </div>
        </div>}
        {knowledgeImport && <div className="border-t border-slate-200 p-5">
          <p className="m-0 text-sm font-semibold text-slate-950">导入预览：{knowledgeImport.file_name}</p>
          <p className="mb-3 mt-1 text-xs text-slate-600">{knowledgeImport.total_rows} 行，{knowledgeImport.valid_rows} 行有效，{knowledgeImport.invalid_rows} 行有错误，状态：{knowledgeImport.status}</p>
          <div className="max-h-56 overflow-auto border border-slate-200 text-xs"><table className="w-full text-left"><thead><tr className="border-b bg-slate-50"><th className="p-2">行</th><th className="p-2">状态</th><th className="p-2">错误</th></tr></thead><tbody>{knowledgeImport.rows.map((row) => <tr key={row.id} className="border-b"><td className="p-2">{row.row_number}</td><td className="p-2">{row.status}</td><td className="p-2">{row.error_message ?? "-"}</td></tr>)}</tbody></table></div>
          {knowledgeImport.status === "previewed" && <div className="mt-3 flex gap-2"><Button onPress={() => updateImport("confirm")} isDisabled={busyId === "knowledge-import"}>确认导入</Button><Button variant="secondary" onPress={() => updateImport("cancel")} isDisabled={busyId === "knowledge-import"}>取消</Button></div>}
        </div>}
      </section>
      <div className="mb-5">
        <Input
          aria-label="操作理由"
          value={reason}
          onChange={(event) => setReason(event.target.value)}
          maxLength={1000}
          placeholder="操作理由"
        />
      </div>
      {operationError && (
        <div
          className="mb-5 border border-rose-200 bg-rose-50 px-4 py-3 text-sm leading-6 text-rose-800"
          role="alert"
        >
          {operationError}
        </div>
      )}
      <section
        className="mb-7 border-y border-slate-200 bg-white"
        aria-labelledby="category-governance-title"
      >
        <header className="border-b border-slate-200 px-5 py-4">
          <h2 id="category-governance-title" className="m-0 text-base font-bold text-slate-950">
            知识分类治理
          </h2>
        </header>
        <div className="divide-y divide-slate-100">
          {categories.length === 0 ? (
            <p className="m-0 px-5 py-6 text-sm text-slate-600">暂无分类申请。</p>
          ) : categories.map((category) => (
            <div key={category.id} className="flex flex-wrap items-center justify-between gap-3 px-5 py-4">
              <div>
                <p className="m-0 text-sm font-semibold text-slate-950">{category.name}</p>
                <p className="mb-0 mt-1 text-xs text-slate-500">状态：{category.status}</p>
              </div>
              {category.status === "pending" ? (
                <div className="flex gap-2">
                  <Button size="sm" isDisabled={busyId === category.id} onPress={() => actOnCategory(category, "enable")}>启用</Button>
                  <Button size="sm" variant="danger" isDisabled={busyId === category.id} onPress={() => actOnCategory(category, "reject")}>驳回</Button>
                </div>
              ) : category.status === "enabled" ? (
                <Button size="sm" variant="danger" isDisabled={busyId === category.id} onPress={() => actOnCategory(category, "disable")}>停用</Button>
              ) : null}
            </div>
          ))}
        </div>
      </section>
      <section className="border-y border-slate-200 bg-white">
        <header className="flex items-center gap-2 border-b border-slate-200 px-5 py-4">
          <FilePlus2 size={18} aria-hidden="true" />
          <h2 className="m-0 text-base font-bold text-slate-950">
            新增学习资源
          </h2>
        </header>
        <form
          className="grid gap-4 p-5 lg:grid-cols-2"
          onSubmit={submitResource}
        >
          <label className="grid gap-1 text-sm text-slate-700">
            标题
            <Input
              aria-label="标题"
              value={resourceForm.title}
              onChange={(event) =>
                setResourceForm({ ...resourceForm, title: event.target.value })
              }
              required
            />
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            来源名称
            <Input
              aria-label="来源名称"
              value={resourceForm.source_name}
              onChange={(event) =>
                setResourceForm({
                  ...resourceForm,
                  source_name: event.target.value,
                })
              }
              required
            />
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            资源类型
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={resourceForm.resource_type}
              onChange={(event) =>
                setResourceForm({
                  ...resourceForm,
                  resource_type: event.target
                    .value as CreateLearningResourceInput["resource_type"],
                })
              }
            >
              <option value="manual">手册</option>
              <option value="prevention">防走失知识</option>
              <option value="team_intro">队伍介绍</option>
              <option value="case_study">案例学习</option>
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            更正上一版本
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={resourceForm.previous_version_id ?? ""}
              onChange={(event) =>
                setResourceForm({
                  ...resourceForm,
                  previous_version_id: event.target.value || null,
                })
              }
            >
              <option value="">新增首版</option>
              {resources
                .filter((resource) => resource.lifecycle.state === "published")
                .map((resource) => (
                  <option key={resource.id} value={resource.id}>
                    {resource.title} v{resource.version}
                  </option>
                ))}
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            可见范围
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={resourceForm.visibility}
              onChange={(event) =>
                setResourceForm({
                  ...resourceForm,
                  visibility: event.target
                    .value as CreateLearningResourceInput["visibility"],
                })
              }
            >
              <option value="learner">新人</option>
              <option value="volunteer">志愿者</option>
              <option value="authenticated">登录用户</option>
              <option value="public">公开</option>
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            来源链接（HTTPS）
            <Input
              aria-label="来源链接（HTTPS）"
              value={resourceForm.source_url ?? ""}
              onChange={(event) =>
                setResourceForm({
                  ...resourceForm,
                  source_url: event.target.value,
                })
              }
            />
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            标签（逗号分隔）
            <Input
              aria-label="标签（逗号分隔）"
              value={resourceTags}
              onChange={(event) => setResourceTags(event.target.value)}
            />
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            分类
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={resourceForm.category_id ?? ""}
              onChange={(event) =>
                setResourceForm({
                  ...resourceForm,
                  category_id: event.target.value || null,
                })
              }
            >
              <option value="">未分类（兼容历史资源）</option>
              {categories
                .filter((category) => category.status === "enabled")
                .map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
            </select>
          </label>
          <TextareaField
            className="lg:col-span-2"
            label="摘要"
            value={resourceForm.summary}
            onChange={(event) =>
              setResourceForm({ ...resourceForm, summary: event.target.value })
            }
            required
          />
          <TextareaField
            className="lg:col-span-2"
            label="正文"
            value={resourceForm.content}
            onChange={(event) =>
              setResourceForm({ ...resourceForm, content: event.target.value })
            }
            minRows={5}
            required
          />
          <TextareaField
            className="lg:col-span-2"
            label="提交理由"
            value={resourceForm.submission_reason}
            onChange={(event) =>
              setResourceForm({
                ...resourceForm,
                submission_reason: event.target.value,
              })
            }
            required
          />
          <div className="lg:col-span-2">
            <Button type="submit" isDisabled={busyId === "new-resource"}>
              {busyId === "new-resource" ? <Spinner size="sm" /> : "提交资源"}
            </Button>
          </div>
        </form>
      </section>
      <section className="mt-7 border-y border-slate-200 bg-white">
        <header className="border-b border-slate-200 px-5 py-4">
          <h2 className="m-0 text-base font-bold text-slate-950">
            资源治理记录
          </h2>
        </header>
        <div className="divide-y divide-slate-100">
          {resources.length === 0 ? (
            <p className="m-0 px-5 py-8 text-sm text-slate-600">暂无资源记录</p>
          ) : (
            resources.map((resource) => (
              <article
                key={resource.id}
                className="flex flex-wrap items-center justify-between gap-3 px-5 py-4"
              >
                <div>
                  <h3 className="m-0 text-sm font-semibold text-slate-950">
                    {resource.title}
                  </h3>
                  <p className="mb-0 mt-1 text-xs text-slate-500">
                    v{resource.version} ·{" "}
                    {transitionLabel(resource.lifecycle.state)} ·{" "}
                    {resource.source_name}
                  </p>
                </div>
                <LifecycleActions
                  lifecycle={resource.lifecycle}
                  currentUserId={user?.id}
                  busy={busyId === resource.id}
                  canAbandonCorrection={Boolean(resource.previous_version_id)}
                  onAction={(action) => actOnResource(resource, action)}
                />
              </article>
            ))
          )}
        </div>
      </section>
      <section className="mt-7 border-y border-slate-200 bg-white">
        <header className="flex items-center gap-2 border-b border-slate-200 px-5 py-4">
          <BookCheck size={18} aria-hidden="true" />
          <h2 className="m-0 text-base font-bold text-slate-950">
            新增学习题目
          </h2>
        </header>
        <form
          className="grid gap-4 p-5 lg:grid-cols-2"
          onSubmit={submitQuestion}
        >
          <label className="grid gap-1 text-sm text-slate-700">
            来源资源
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={questionForm.source_resource_id}
              onChange={(event) =>
                setQuestionForm({
                  ...questionForm,
                  source_resource_id: event.target.value,
                })
              }
              required
            >
              <option value="">选择资源</option>
              {resources
                .filter(
                  (resource) =>
                    resource.lifecycle.state === "published" &&
                    (!correctionSourceResourceId ||
                      resource.id === correctionSourceResourceId),
                )
                .map((resource) => (
                  <option key={resource.id} value={resource.id}>
                    {resource.title}
                  </option>
                ))}
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            更正上一版本
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={questionForm.previous_version_id ?? ""}
              onChange={(event) => {
                const previousVersionId = event.target.value || null;
                const previousQuestion = previousVersionId
                  ? questions.find(
                      (question) => question.id === previousVersionId,
                    )
                  : null;
                setQuestionForm({
                  ...questionForm,
                  previous_version_id: previousVersionId,
                  source_resource_id:
                    previousQuestion?.source_resource_id ??
                    questionForm.source_resource_id,
                });
              }}
            >
              <option value="">新增首版</option>
              {questions
                .filter((question) => question.lifecycle.state === "published")
                .map((question) => (
                  <option key={question.id} value={question.id}>
                    {question.prompt} v{question.version}
                  </option>
                ))}
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            正确选项编号
            <Input
              aria-label="正确选项编号"
              value={questionForm.correct_option_id}
              onChange={(event) =>
                setQuestionForm({
                  ...questionForm,
                  correct_option_id: event.target.value,
                })
              }
              required
            />
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            题目类型
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={questionForm.question_type}
              onChange={(event) =>
                setQuestionForm({
                  ...questionForm,
                  question_type: event.target
                    .value as CreateLearningQuestionInput["question_type"],
                })
              }
            >
              <option value="single_choice">选择题</option>
              <option value="true_false">判断题</option>
              <option value="scenario">情景题</option>
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            难度
            <select
              className="h-10 rounded-md border border-slate-300 bg-white px-3"
              value={questionForm.difficulty}
              onChange={(event) =>
                setQuestionForm({
                  ...questionForm,
                  difficulty: event.target
                    .value as CreateLearningQuestionInput["difficulty"],
                })
              }
            >
              <option value="basic">基础</option>
              <option value="intermediate">进阶</option>
              <option value="advanced">高级</option>
            </select>
          </label>
          <TextareaField
            className="lg:col-span-2"
            label="题干"
            value={questionForm.prompt}
            onChange={(event) =>
              setQuestionForm({ ...questionForm, prompt: event.target.value })
            }
            required
          />
          <TextareaField
            className="lg:col-span-2"
            label="选项（每行：编号|文本）"
            value={optionLines}
            onChange={(event) => setOptionLines(event.target.value)}
            minRows={3}
            required
          />
          <label className="grid gap-1 text-sm text-slate-700">
            标签（逗号分隔）
            <Input
              aria-label="标签（逗号分隔）"
              value={questionTags}
              onChange={(event) => setQuestionTags(event.target.value)}
            />
          </label>
          <TextareaField
            label="解析"
            value={questionForm.explanation}
            onChange={(event) =>
              setQuestionForm({
                ...questionForm,
                explanation: event.target.value,
              })
            }
            required
          />
          <TextareaField
            className="lg:col-span-2"
            label="提交理由"
            value={questionForm.submission_reason}
            onChange={(event) =>
              setQuestionForm({
                ...questionForm,
                submission_reason: event.target.value,
              })
            }
            required
          />
          <div className="lg:col-span-2">
            <Button type="submit" isDisabled={busyId === "new-question"}>
              {busyId === "new-question" ? <Spinner size="sm" /> : "提交题目"}
            </Button>
          </div>
        </form>
      </section>
      <section className="mt-7 border-y border-slate-200 bg-white">
        <header className="border-b border-slate-200 px-5 py-4">
          <h2 className="m-0 text-base font-bold text-slate-950">
            题目治理记录
          </h2>
        </header>
        <div className="divide-y divide-slate-100">
          {questions.length === 0 ? (
            <p className="m-0 px-5 py-8 text-sm text-slate-600">暂无题目记录</p>
          ) : (
            questions.map((question) => (
              <article
                key={question.id}
                className="flex flex-wrap items-center justify-between gap-3 px-5 py-4"
              >
                <div>
                  <h3 className="m-0 text-sm font-semibold text-slate-950">
                    {question.prompt}
                  </h3>
                  <p className="mb-0 mt-1 text-xs text-slate-500">
                    v{question.version} ·{" "}
                    {transitionLabel(question.lifecycle.state)}
                  </p>
                </div>
                <LifecycleActions
                  lifecycle={question.lifecycle}
                  currentUserId={user?.id}
                  busy={busyId === question.id}
                  canAbandonCorrection={Boolean(question.previous_version_id)}
                  onAction={(action) => actOnQuestion(question, action)}
                />
              </article>
            ))
          )}
        </div>
      </section>
    </main>
  );
}
