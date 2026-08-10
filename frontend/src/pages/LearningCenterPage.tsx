import { Button, Chip, Input, Spinner } from "@heroui/react";
import { BookOpen, Send, WifiOff } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  askKnowledge,
  getPublicPreventionCard,
  listLearningCategories,
  listLearningQuestions,
  listLearningResources,
  submitLearningCategoryProposal,
  submitLearningAnswer,
  submitLearningResourceDraft,
} from "../api/learning";
import type {
  KnowledgeAnswer,
  CreateLearningResourceInput,
  LearningCategory,
  LearningQuestion,
  LearningResource,
} from "../api/learning";
import { ApiClientError } from "../api/client";
import { useAuth } from "../auth/useAuth";
import { ErrorState, LoadingState } from "../components/ContentState";
import { usePreventionCacheReady } from "../offline/prevention-cache";

const resourceTypeLabels: Record<LearningResource["resource_type"], string> = {
  team_intro: "协作指引",
  manual: "操作手册",
  prevention: "防走失知识",
  case_study: "脱敏案例",
};

function messageFrom(cause: unknown) {
  return cause instanceof ApiClientError
    ? cause.message
    : "暂时无法完成学习中心操作，请稍后重试。";
}

function isMissingCategoryEndpoint(cause: unknown) {
  return (
    cause instanceof ApiClientError &&
    cause.status === 404 &&
    cause.code === "not_found"
  );
}

export function LearningCenterPage() {
  const { token, user } = useAuth();
  const preventionCacheReady = usePreventionCacheReady();
  const [resources, setResources] = useState<LearningResource[]>([]);
  const [questions, setQuestions] = useState<LearningQuestion[]>([]);
  const [categories, setCategories] = useState<LearningCategory[]>([]);
  const [selectedCategory, setSelectedCategory] = useState("");
  const [selectedTag, setSelectedTag] = useState("");
  const [draftTags, setDraftTags] = useState("");
  const [draftError, setDraftError] = useState("");
  const [isSubmittingDraft, setIsSubmittingDraft] = useState(false);
  const [draft, setDraft] = useState<CreateLearningResourceInput>({
    title: "", summary: "", content: "", resource_type: "manual", tags: [], category_id: null,
    source_name: "", source_url: null, visibility: "learner", effective_at: new Date().toISOString(),
    permitted_use: "training", submission_reason: "",
  });
  const [categoryProposal, setCategoryProposal] = useState("");
  const [categoryProposalReason, setCategoryProposalReason] = useState("");
  const [preventionCard, setPreventionCard] = useState<LearningResource | null>(
    null,
  );
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [answer, setAnswer] = useState<KnowledgeAnswer | null>(null);
  const [knowledgeError, setKnowledgeError] = useState("");
  const [isAsking, setIsAsking] = useState(false);
  const [results, setResults] = useState<
    Record<string, { isCorrect: boolean; explanation: string }>
  >({});
  const [answeringQuestion, setAnsweringQuestion] = useState("");

  const load = useCallback(async () => {
    if (!token) {
      setError("登录状态不可用，请重新登录后访问学习中心。");
      setIsLoading(false);
      return;
    }
    setIsLoading(true);
    setError("");
    try {
      const [nextResources, nextQuestions] = await Promise.all([
        listLearningResources(token, { category_id: selectedCategory, tag: selectedTag }),
        listLearningQuestions(token),
      ]);
      setResources(nextResources);
      setQuestions(nextQuestions);
      // Categories are progressive enhancement for legacy deployments. A
      // missing category endpoint must not hide already published resources.
      const nextCategories =
        typeof listLearningCategories === "function"
          ? await listLearningCategories(token).catch((cause) => {
              if (isMissingCategoryEndpoint(cause)) return [];
              throw cause;
            })
          : [];
      setCategories(nextCategories);
      try {
        setPreventionCard(await getPublicPreventionCard());
      } catch (cause) {
        if (!(cause instanceof ApiClientError) || cause.status !== 404)
          throw cause;
        setPreventionCard(null);
      }
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setIsLoading(false);
    }
  }, [selectedCategory, selectedTag, token]);

  useEffect(() => {
    void load();
  }, [load]);
  if (isLoading) return <LoadingState label="正在加载学习资源" />;
  if (error) return <ErrorState message={error} onRetry={() => void load()} />;

  const answerQuestion = (questionId: string, optionId: string) => {
    if (!token) return;
    setAnsweringQuestion(questionId);
    void submitLearningAnswer(token, questionId, optionId)
      .then((result) =>
        setResults((current) => ({
          ...current,
          [questionId]: {
            isCorrect: result.is_correct,
            explanation: result.explanation,
          },
        })),
      )
      .catch((cause) => setError(messageFrom(cause)))
      .finally(() => setAnsweringQuestion(""));
  };

  const submitDraft = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token) return;
    setDraftError("");
    setIsSubmittingDraft(true);
    void submitLearningResourceDraft(token, {
      ...draft,
      tags: draftTags.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean),
      source_url: draft.source_url?.trim() || null,
    })
      .then(() => {
        setDraft({
          ...draft, title: "", summary: "", content: "", tags: [], category_id: null,
          source_name: "", source_url: null, submission_reason: "", effective_at: new Date().toISOString(),
        });
        setDraftTags("");
        setDraftError("草稿已提交，等待独立去标识、审核与发布流程。");
      })
      .catch((cause) => setDraftError(messageFrom(cause)))
      .finally(() => setIsSubmittingDraft(false));
  };

  const proposeCategory = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!token || !categoryProposal.trim() || !categoryProposalReason.trim()) return;
    setDraftError("");
    void submitLearningCategoryProposal(token, categoryProposal, categoryProposalReason)
      .then(() => {
        setCategoryProposal("");
        setCategoryProposalReason("");
        setDraftError("分类申请已提交，管理员启用后即可在资源草稿中选择。");
        return load();
      })
      .catch((cause) => setDraftError(messageFrom(cause)));
  };

  const availableTags = Array.from(new Set(resources.flatMap((resource) => resource.tags))).sort();

  return (
    <main className="mx-auto w-full max-w-7xl px-4 py-7 sm:px-6 lg:px-10 lg:py-10">
      <header className="mb-7 flex items-start gap-3">
        <span className="grid size-11 place-items-center rounded-md bg-brand-100 text-brand-700">
          <BookOpen aria-hidden="true" />
        </span>
        <div>
          <h1 className="m-0 text-2xl font-bold text-slate-950">学习中心</h1>
          <p className="mb-0 mt-1 text-sm text-slate-600">
            仅展示已发布、可追溯的教材；内容不能替代负责人审核或现场指令。
          </p>
        </div>
      </header>
      <section
        className="mb-7 border-y border-emerald-200 bg-emerald-50"
        aria-labelledby="offline-prevention-title"
      >
        <header className="flex items-center justify-between gap-3 border-b border-emerald-200 px-5 py-4">
          <div className="flex items-center gap-2">
            <WifiOff size={18} aria-hidden="true" />
            <h2
              id="offline-prevention-title"
              className="m-0 text-base font-bold text-slate-950"
            >
              离线防走失知识卡
            </h2>
          </div>
          <Chip size="sm" variant="soft">
            <Chip.Label>
              {preventionCard
                ? preventionCacheReady
                  ? "可离线使用"
                  : "仅可在线查看"
                : "等待发布"}
            </Chip.Label>
          </Chip>
        </header>
        {preventionCard ? (
          <article className="px-5 py-4">
            <h3 className="m-0 text-sm font-semibold text-slate-950">
              {preventionCard.title}
            </h3>
            <p className="mb-0 mt-2 whitespace-pre-wrap text-sm leading-6 text-slate-700">
              {preventionCard.content}
            </p>
            <p className="mb-0 mt-3 text-xs text-slate-600">
              来源：{preventionCard.source_name} · v{preventionCard.version} ·
              审核状态：已发布 · 生效时间：
              {formatDate(preventionCard.effective_at)}
            </p>
            {!preventionCacheReady && (
              <p className="mb-0 mt-2 text-xs text-amber-800" role="status">
                离线缓存尚未就绪，请保持联网查看。
              </p>
            )}
          </article>
        ) : (
          <p className="m-0 px-5 py-6 text-sm leading-6 text-slate-600">
            负责人尚未发布可离线使用的防走失知识卡。该卡发布并加载成功后，生产环境会保留最后一个已审核版本供离线查看。
          </p>
        )}
      </section>
      <section
        className="mb-7 border-y border-slate-200 bg-white"
        aria-labelledby="learning-resources-title"
      >
        <header className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
          <h2
            id="learning-resources-title"
            className="m-0 text-base font-bold text-slate-950"
          >
            手册与知识卡
          </h2>
          <Chip size="sm" variant="soft">
            <Chip.Label>{resources.length} 项</Chip.Label>
          </Chip>
        </header>
        <div className="grid gap-3 border-b border-slate-100 px-5 py-4 sm:grid-cols-2">
          <label className="grid gap-1 text-sm text-slate-700">
            分类筛选
            <select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={selectedCategory} onChange={(event) => setSelectedCategory(event.target.value)}>
              <option value="">全部分类（含历史未分类）</option>
              {categories.map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}
            </select>
          </label>
          <label className="grid gap-1 text-sm text-slate-700">
            标签筛选
            <select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={selectedTag} onChange={(event) => setSelectedTag(event.target.value)}>
              <option value="">全部标签</option>
              {availableTags.map((tag) => <option key={tag} value={tag}>{tag}</option>)}
            </select>
          </label>
        </div>
        {resources.length === 0 ? (
          <p className="m-0 px-5 py-8 text-sm text-slate-600">
            暂无已发布学习资源。资源需经过审核后才会出现在这里。
          </p>
        ) : (
          <div className="divide-y divide-slate-100">
            {resources.map((resource) => (
              <article key={resource.id} className="px-5 py-4">
                <div className="flex flex-wrap items-center gap-2">
                  <h3 className="m-0 text-sm font-semibold text-slate-950">
                    {resource.title}
                  </h3>
                  <Chip size="sm" variant="soft">
                    <Chip.Label>
                      {resourceTypeLabels[resource.resource_type]}
                    </Chip.Label>
                  </Chip>
                  <Chip size="sm" variant="soft">
                    <Chip.Label>v{resource.version}</Chip.Label>
                  </Chip>
                  {resource.category && (
                    <Chip size="sm" variant="soft">
                      <Chip.Label>{resource.category.name}</Chip.Label>
                    </Chip>
                  )}
                  {resource.tags.map((tag) => (
                    <Chip key={tag} size="sm" variant="soft">
                      <Chip.Label>#{tag}</Chip.Label>
                    </Chip>
                  ))}
                </div>
                <p className="mb-0 mt-1 text-sm text-slate-600">
                  {resource.summary}
                </p>
                <p className="mb-0 mt-3 whitespace-pre-wrap text-sm leading-6 text-slate-700">
                  {resource.content}
                </p>
                <p className="mb-0 mt-2 text-xs text-slate-500">
                  来源：{resource.source_name}
                  {resource.source_url && (
                    <>
                      {" "}
                      ·{" "}
                      <a
                        className="text-brand-700 underline underline-offset-2"
                        href={resource.source_url}
                        target="_blank"
                        rel="noreferrer"
                      >
                        查看原始来源
                      </a>
                    </>
                  )}{" "}
                  · 审核状态：已发布 · 生效时间：
                  {formatDate(resource.effective_at)}
                </p>
              </article>
            ))}
          </div>
        )}
      </section>
      {user?.account_type === "learner" && (
        <section className="mb-7 border-y border-slate-200 bg-white" aria-labelledby="contribute-learning-title">
          <header className="border-b border-slate-200 px-5 py-4">
            <h2 id="contribute-learning-title" className="m-0 text-base font-bold text-slate-950">提交学习资源草稿</h2>
            <p className="mb-0 mt-1 text-sm text-slate-600">草稿不会直接展示；它必须经过独立去标识、审核和发布。</p>
          </header>
          <form className="grid gap-3 p-5 lg:grid-cols-2" onSubmit={submitDraft}>
            <label className="grid gap-1 text-sm text-slate-700">标题<Input aria-label="草稿标题" value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} required /></label>
            <label className="grid gap-1 text-sm text-slate-700">来源名称<Input aria-label="草稿来源名称" value={draft.source_name} onChange={(event) => setDraft({ ...draft, source_name: event.target.value })} required /></label>
            <label className="grid gap-1 text-sm text-slate-700">分类
              <select className="h-10 rounded-md border border-slate-300 bg-white px-3" value={draft.category_id ?? ""} onChange={(event) => setDraft({ ...draft, category_id: event.target.value || null })}>
                <option value="">未分类</option>{categories.map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}
              </select>
            </label>
            <label className="grid gap-1 text-sm text-slate-700">标签（逗号分隔）<Input aria-label="草稿标签" value={draftTags} onChange={(event) => setDraftTags(event.target.value)} /></label>
            <label className="grid gap-1 text-sm text-slate-700 lg:col-span-2">摘要<textarea className="min-h-20 rounded-md border border-slate-300 px-3 py-2" value={draft.summary} onChange={(event) => setDraft({ ...draft, summary: event.target.value })} required /></label>
            <label className="grid gap-1 text-sm text-slate-700 lg:col-span-2">正文<textarea className="min-h-32 rounded-md border border-slate-300 px-3 py-2" value={draft.content} onChange={(event) => setDraft({ ...draft, content: event.target.value })} required /></label>
            <label className="grid gap-1 text-sm text-slate-700 lg:col-span-2">提交理由<textarea className="min-h-20 rounded-md border border-slate-300 px-3 py-2" value={draft.submission_reason} onChange={(event) => setDraft({ ...draft, submission_reason: event.target.value })} required /></label>
            <div className="lg:col-span-2"><Button type="submit" isDisabled={isSubmittingDraft}>{isSubmittingDraft ? <Spinner size="sm" /> : "提交草稿"}</Button></div>
          </form>
          <form className="grid gap-3 border-t border-slate-100 p-5 lg:grid-cols-2" onSubmit={proposeCategory}>
            <p className="m-0 text-sm font-semibold text-slate-950 lg:col-span-2">没有合适的分类？提交分类申请</p>
            <Input aria-label="申请分类名称" value={categoryProposal} onChange={(event) => setCategoryProposal(event.target.value)} placeholder="分类名称" required />
            <Input aria-label="申请分类理由" value={categoryProposalReason} onChange={(event) => setCategoryProposalReason(event.target.value)} placeholder="申请理由" required />
            <div className="lg:col-span-2"><Button type="submit" variant="secondary">申请分类</Button></div>
          </form>
          {draftError && <p className="m-0 px-5 pb-5 text-sm text-slate-700" role="status">{draftError}</p>}
        </section>
      )}
      <div className="grid gap-7 lg:grid-cols-2">
        <section
          className="border-y border-slate-200 bg-white"
          aria-labelledby="learning-questions-title"
        >
          <header className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
            <h2
              id="learning-questions-title"
              className="m-0 text-base font-bold text-slate-950"
            >
              理论题库
            </h2>
            <Chip size="sm" variant="soft">
              <Chip.Label>{questions.length} 题</Chip.Label>
            </Chip>
          </header>
          {questions.length === 0 ? (
            <p className="m-0 px-5 py-8 text-sm text-slate-600">
              暂无已发布题目。题目和解析需由教材负责人录入并审核。
            </p>
          ) : (
            <div className="divide-y divide-slate-100">
              {questions.map((question) => (
                <article key={question.id} className="px-5 py-4">
                  <p className="m-0 text-sm font-semibold leading-6 text-slate-950">
                    {question.prompt}
                  </p>
                  <div className="mt-3 grid gap-2">
                    {question.options.map((option) => (
                      <Button
                        key={option.id}
                        className="justify-start! text-left"
                        variant="secondary"
                        isDisabled={
                          answeringQuestion === question.id ||
                          Boolean(results[question.id])
                        }
                        onPress={() => answerQuestion(question.id, option.id)}
                      >
                        {option.text}
                      </Button>
                    ))}
                  </div>
                  {results[question.id] && (
                    <p
                      className={`mb-0 mt-3 text-sm ${results[question.id].isCorrect ? "text-emerald-700" : "text-amber-700"}`}
                    >
                      {results[question.id].isCorrect
                        ? "回答正确。"
                        : "回答不正确。"}{" "}
                      {results[question.id].explanation}
                    </p>
                  )}
                </article>
              ))}
            </div>
          )}
        </section>
        <section
          className="border-y border-slate-200 bg-white"
          aria-labelledby="knowledge-ask-title"
        >
          <header className="border-b border-slate-200 px-5 py-4">
            <h2
              id="knowledge-ask-title"
              className="m-0 text-base font-bold text-slate-950"
            >
              新人问答
            </h2>
          </header>
          <form
            className="flex gap-2 p-5"
            onSubmit={(event) => {
              event.preventDefault();
              if (!token || !prompt.trim()) return;
              setIsAsking(true);
              setAnswer(null);
              setKnowledgeError("");
              void askKnowledge(token, prompt)
                .then(setAnswer)
                .catch((cause) => setKnowledgeError(messageFrom(cause)))
                .finally(() => setIsAsking(false));
            }}
          >
            <Input
              aria-label="输入学习问题"
              value={prompt}
              maxLength={1000}
              onChange={(event) => setPrompt(event.target.value)}
              placeholder="输入一个和已审核教材有关的问题"
            />
            <Button
              type="submit"
              isIconOnly
              aria-label="提交问题"
              isDisabled={isAsking || !prompt.trim()}
            >
              {isAsking ? (
                <Spinner size="sm" />
              ) : (
                <Send size={16} aria-hidden="true" />
              )}
            </Button>
          </form>
          {knowledgeError && (
            <p className="m-0 px-5 pb-4 text-sm text-amber-800" role="status">
              问答暂时不可用，已发布资料仍可查看。请稍后重试。
            </p>
          )}
          {answer ? (
            <div className="mx-5 mb-5 rounded-md border border-slate-200 bg-slate-50 p-4">
              <p className="m-0 whitespace-pre-wrap text-sm leading-6 text-slate-700">
                {answer.answer}
              </p>
              <p className="mb-0 mt-3 text-xs text-slate-500">
                资料状态：
                {answer.certainty === "source_backed"
                  ? "已审核资料支持"
                  : "资料不足，无法形成行动建议"}
              </p>
              <p className="mb-0 mt-2 text-xs text-slate-500">
                {answer.human_review_notice}
              </p>
              {answer.sources.length > 0 && (
                <p className="mb-0 mt-2 text-xs text-slate-500">
                  引用来源（均为已审核发布）：
                  {answer.sources
                    .map((source) => `${source.title} v${source.version}`)
                    .join("、")}
                </p>
              )}
            </div>
          ) : (
            <p className="m-0 px-5 pb-8 text-sm text-slate-500">
              答案只来自当前账号可见的已发布资源。
            </p>
          )}
        </section>
      </div>
    </main>
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : date.toLocaleString("zh-CN", { hour12: false });
}
