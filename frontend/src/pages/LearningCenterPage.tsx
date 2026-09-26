import { Button, Chip, Input, Spinner } from "@heroui/react";
import { Bookmark, BookOpen, Clock3, WifiOff } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  getPublicPreventionCard,
  listLearningCategories,
  listLearningQuestions,
  listLearningResources,
  listLearningAnswers,
  listWrongLearningAnswers,
  getLearningProgress,
  searchLearningKnowledge,
  askKnowledge,
  downloadKnowledgeImage,
  downloadKnowledgeAttachment,
  submitLearningCategoryProposal,
  submitLearningAnswer,
  submitLearningResourceDraft,
} from "../api/learning";
import type {
  CreateLearningResourceInput,
  LearningCategory,
  LearningQuestion,
  LearningResource,
  LearningAnswerHistory,
  LearningProgress,
  KnowledgeAnswer,
  KnowledgeImage,
  KnowledgeSearchResult,
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
  const [results, setResults] = useState<
    Record<string, { isCorrect: boolean; explanation: string; score: number; maxScore: number }>
  >({});
  const [answerHistory, setAnswerHistory] = useState<LearningAnswerHistory[]>([]);
  const [wrongAnswers, setWrongAnswers] = useState<LearningAnswerHistory[]>([]);
  const [progress, setProgress] = useState<LearningProgress | null>(null);
  const [answeringQuestion, setAnsweringQuestion] = useState("");
  const [knowledgeQuery, setKnowledgeQuery] = useState("");
  const [knowledgeCategory, setKnowledgeCategory] = useState("");
  const [knowledgeTag, setKnowledgeTag] = useState("");
  const [knowledgeResults, setKnowledgeResults] = useState<KnowledgeSearchResult[]>([]);
  const [knowledgeCatalogue, setKnowledgeCatalogue] = useState<KnowledgeSearchResult[]>([]);
  const [recentKnowledgeIds, setRecentKnowledgeIds] = useState<string[]>([]);
  const [favoriteKnowledgeIds, setFavoriteKnowledgeIds] = useState<string[]>([]);
  const [knowledgeAnswer, setKnowledgeAnswer] = useState<KnowledgeAnswer | null>(null);
  const [knowledgeBusy, setKnowledgeBusy] = useState(false);
  const [knowledgeMessage, setKnowledgeMessage] = useState("");
  const [knowledgeImageUrls, setKnowledgeImageUrls] = useState<Record<string, string>>({});
  const [knowledgeImageFailures, setKnowledgeImageFailures] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (!user?.id) return;
    const readIds = (kind: string): string[] => {
      try {
        const value: unknown = JSON.parse(localStorage.getItem(`angui:knowledge:${user.id}:${kind}`) ?? "[]");
        return Array.isArray(value) ? value.filter((id): id is string => typeof id === "string").slice(0, 30) : [];
      } catch {
        return [];
      }
    };
    setRecentKnowledgeIds(readIds("recent"));
    setFavoriteKnowledgeIds(readIds("favorite"));
  }, [user?.id]);

  const rememberKnowledge = (id: string) => {
    if (!user?.id) return;
    setRecentKnowledgeIds((current) => {
      const next = [id, ...current.filter((value) => value !== id)].slice(0, 30);
      localStorage.setItem(`angui:knowledge:${user.id}:recent`, JSON.stringify(next));
      return next;
    });
  };
  const toggleKnowledgeFavorite = (id: string) => {
    if (!user?.id) return;
    setFavoriteKnowledgeIds((current) => {
      const next = current.includes(id) ? current.filter((value) => value !== id) : [id, ...current].slice(0, 30);
      localStorage.setItem(`angui:knowledge:${user.id}:favorite`, JSON.stringify(next));
      return next;
    });
  };
  const showSavedKnowledge = (ids: string[]) => {
    const visible = new Map(knowledgeCatalogue.map((item) => [item.knowledge_item_id, item]));
    setKnowledgeResults(ids.flatMap((id) => visible.has(id) ? [visible.get(id)!] : []));
    setKnowledgeAnswer(null);
    setKnowledgeMessage("");
  };

  useEffect(() => {
    const images = new Map<string, { itemId: string; imageId: string }>();
    const sources = [...knowledgeResults, ...(knowledgeAnswer?.sources ?? [])];
    for (const source of sources) {
      for (const image of source.images) {
        images.set(image.id, {
          itemId: source.knowledge_item_id,
          imageId: image.id,
        });
      }
    }
    const controller = new AbortController();
    let active = true;
    const createdUrls: string[] = [];
    setKnowledgeImageFailures({});
    setKnowledgeImageUrls((current) => {
      Object.values(current).forEach((url) => URL.revokeObjectURL(url));
      return {};
    });

    if (!token || images.size === 0) {
      return () => {
        active = false;
        controller.abort();
      };
    }

    void Promise.all(
      [...images.values()].map(async ({ itemId, imageId }) => {
        try {
          const blob = await downloadKnowledgeImage(token, itemId, imageId, {
            signal: controller.signal,
          });
          const url = URL.createObjectURL(blob);
          if (!active) {
            URL.revokeObjectURL(url);
            return;
          }
          createdUrls.push(url);
          setKnowledgeImageUrls((current) => ({ ...current, [imageId]: url }));
        } catch {
          if (active && !controller.signal.aborted) {
            setKnowledgeImageFailures((current) => ({ ...current, [imageId]: true }));
          }
        }
      }),
    );

    return () => {
      active = false;
      controller.abort();
      createdUrls.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [knowledgeAnswer, knowledgeResults, token]);

  const searchKnowledge = () => {
    if (!token) return;
    setKnowledgeBusy(true); setKnowledgeAnswer(null); setKnowledgeMessage("");
    void searchLearningKnowledge(token, { query: knowledgeQuery, category_id: knowledgeCategory, tag: knowledgeTag }).then((response) => {
      setKnowledgeResults(response.results);
      if (response.results.length === 0) setKnowledgeMessage("没有匹配资料。可尝试更具体的关键词或切换分类。");
    }).catch((cause) => setKnowledgeMessage(messageFrom(cause))).finally(() => setKnowledgeBusy(false));
  };
  const askFromKnowledge = () => {
    if (!token || !knowledgeQuery.trim()) return;
    setKnowledgeBusy(true); setKnowledgeMessage("");
    void askKnowledge(token, knowledgeQuery, { category: knowledgeCategory, tag: knowledgeTag }).then(setKnowledgeAnswer).catch((cause) => setKnowledgeMessage(messageFrom(cause))).finally(() => setKnowledgeBusy(false));
  };

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
      const catalogue = await Promise.resolve(searchLearningKnowledge(token)).catch(() => null);
      setKnowledgeCatalogue(catalogue?.results ?? []);
      if (typeof listLearningAnswers === "function" && typeof listWrongLearningAnswers === "function" && typeof getLearningProgress === "function") {
        const [history, wrong, nextProgress] = await Promise.all([
          listLearningAnswers(token),
          listWrongLearningAnswers(token),
          getLearningProgress(token),
        ]);
        setAnswerHistory(history);
        setWrongAnswers(wrong);
        setProgress(nextProgress);
        const latest = new Map<string, LearningAnswerHistory>();
        for (const answer of history) if (!latest.has(answer.question_id)) latest.set(answer.question_id, answer);
        setResults(Object.fromEntries([...latest.values()].map((answer) => [answer.question_id, {
          isCorrect: answer.is_correct,
          explanation: answer.question_snapshot?.explanation ?? "历史解析不可用",
          score: answer.score,
          maxScore: answer.max_score,
        }])));
      }
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
      .then((result) => {
        setResults((current) => ({
          ...current,
          [questionId]: {
            isCorrect: result.is_correct,
            explanation: result.explanation,
            score: result.score,
            maxScore: result.max_score,
          },
        }));
        void Promise.all([listLearningAnswers(token), listWrongLearningAnswers(token), getLearningProgress(token)])
          .then(([history, wrong, nextProgress]) => { setAnswerHistory(history); setWrongAnswers(wrong); setProgress(nextProgress); })
          .catch((cause) => setError(messageFrom(cause)));
      })
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
  const openKnowledgeAttachment = (itemId: string, attachmentId: string, fileName: string) => {
    if (!token) return;
    void downloadKnowledgeAttachment(token, itemId, attachmentId)
      .then((blob) => { const url = URL.createObjectURL(blob); const link = document.createElement("a"); link.href = url; link.download = fileName; link.click(); window.setTimeout(() => URL.revokeObjectURL(url), 60_000); })
      .catch((cause) => setKnowledgeMessage(messageFrom(cause)));
  };
  const knowledgeCategories = Array.from(new Set(knowledgeCatalogue.map((result) => result.category).filter(Boolean))).sort();
  const knowledgeTags = Array.from(new Set(knowledgeCatalogue.flatMap((result) => result.keywords))).sort();
  const activeQuestionIds = new Set(questions.map((question) => question.id));
  const recommendedQuestion = wrongAnswers.find((answer) => activeQuestionIds.has(answer.question_id))?.question_id
    ?? questions.find((question) => !answerHistory.some((answer) => answer.question_id === question.id))?.id;
  const recommendedKnowledge = knowledgeCatalogue.find((item) => !recentKnowledgeIds.includes(item.knowledge_item_id));
  const jumpToQuestion = () => {
    if (recommendedQuestion) document.getElementById(`learning-question-${recommendedQuestion}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
  };
  const renderKnowledgeImages = (
    title: string,
    images: KnowledgeImage[],
  ) => {
    if (images.length === 0) return null;
    return (
      <div className="mt-3 flex flex-wrap gap-3" aria-label={`${title} 的附图`}>
        {images.map((image, index) => {
          const url = knowledgeImageUrls[image.id];
          if (url) {
            return (
              <a key={image.id} href={url} target="_blank" rel="noreferrer" className="block">
                <img
                  src={url}
                  alt={`${title} 附图 ${index + 1}`}
                  className="h-24 w-36 rounded-md border border-slate-200 object-cover"
                />
              </a>
            );
          }
          return (
            <p key={image.id} className="m-0 text-xs text-slate-500">
              {knowledgeImageFailures[image.id]
                ? "附图暂时无法加载。"
                : "正在加载附图…"}
            </p>
          );
        })}
      </div>
    );
  };

  const knowledgeAnswerStatus = knowledgeAnswer && {
    source_backed: "AI Gateway 已基于下列已发布资料生成回答。",
    rule_based: "当前未配置或无法连接可用的 AI Gateway；以下是命中资料原文拼接，不是模型生成回答。",
    insufficient_sources: "没有足够的已发布资料，系统不会生成回答。",
  }[knowledgeAnswer.certainty];

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
      <section className="mb-7 border-y border-slate-200 bg-white" aria-labelledby="knowledge-search-title">
        <header className="border-b border-slate-200 px-5 py-4"><h2 id="knowledge-search-title" className="m-0 text-base font-bold text-slate-950">知识检索与问答</h2><p className="mb-0 mt-1 text-sm text-slate-600">先按关键词匹配已发布、已脱敏的资料；问答只使用本次命中的资料作为上下文。</p></header>
        <div className="grid gap-2 p-5 sm:grid-cols-[1fr_10rem_10rem_auto_auto]">
          <Input aria-label="搜索知识" value={knowledgeQuery} onChange={(event) => setKnowledgeQuery(event.target.value)} placeholder="输入关键词或问题" maxLength={1000} />
          <select aria-label="知识分类筛选" className="h-10 rounded-md border border-slate-300 bg-white px-3" value={knowledgeCategory} onChange={(event) => setKnowledgeCategory(event.target.value)}><option value="">全部分类</option>{knowledgeCategories.map((category) => <option key={category} value={category}>{category}</option>)}</select>
          <select aria-label="知识标签筛选" className="h-10 rounded-md border border-slate-300 bg-white px-3" value={knowledgeTag} onChange={(event) => setKnowledgeTag(event.target.value)}><option value="">全部标签</option>{knowledgeTags.map((tag) => <option key={tag} value={tag}>{tag}</option>)}</select>
          <Button onPress={searchKnowledge} isDisabled={knowledgeBusy}>{knowledgeBusy ? <Spinner size="sm" /> : "搜索"}</Button>
          <Button variant="secondary" onPress={askFromKnowledge} isDisabled={knowledgeBusy || !knowledgeQuery.trim()}>基于资料问答</Button>
        </div>
        <div className="flex flex-wrap gap-2 px-5 pb-4">
          <Button size="sm" variant="secondary" onPress={() => showSavedKnowledge(recentKnowledgeIds)} isDisabled={!recentKnowledgeIds.length}><Clock3 size={15} />最近查看</Button>
          <Button size="sm" variant="secondary" onPress={() => showSavedKnowledge(favoriteKnowledgeIds)} isDisabled={!favoriteKnowledgeIds.length}><Bookmark size={15} />我的收藏</Button>
        </div>
        {knowledgeMessage && <p className="m-0 px-5 pb-4 text-sm text-amber-800" role="status">{knowledgeMessage}</p>}
        {knowledgeResults.length > 0 && <div className="divide-y divide-slate-100">{knowledgeResults.map((result) =>
          <article key={result.knowledge_item_id} className="px-5 py-4">
            <div className="flex items-start justify-between gap-2"><h3 className="m-0 text-sm font-semibold">{result.title}</h3><button type="button" title={favoriteKnowledgeIds.includes(result.knowledge_item_id) ? "取消收藏" : "收藏资料"} aria-label={favoriteKnowledgeIds.includes(result.knowledge_item_id) ? "取消收藏" : "收藏资料"} aria-pressed={favoriteKnowledgeIds.includes(result.knowledge_item_id)} className="shrink-0 text-slate-600 hover:text-brand-700" onClick={() => toggleKnowledgeFavorite(result.knowledge_item_id)}><Bookmark size={17} fill={favoriteKnowledgeIds.includes(result.knowledge_item_id) ? "currentColor" : "none"} /></button></div>
            <p className="mb-0 mt-1 text-sm text-slate-600">{result.summary}</p>
            <p className="mb-0 mt-2 text-xs text-slate-500">{Boolean(result.matched_fields?.length) && `匹配字段：${result.matched_fields.join("、")} · `}#{result.category} · {result.keywords.map((tag) => `#${tag}`).join(" ")}</p>
            <details className="mt-3 text-sm" onToggle={(event) => { if (event.currentTarget.open) rememberKnowledge(result.knowledge_item_id); }}><summary className="cursor-pointer text-brand-700">查看正文与附件</summary><p className="mt-2 whitespace-pre-wrap leading-6 text-slate-700">{result.content}</p>{result.attachments.length > 0 && <ul className="mt-2 list-disc pl-5 text-xs text-slate-600">{result.attachments.map((attachment) => <li key={attachment.id}><button type="button" className="text-brand-700 underline" onClick={() => openKnowledgeAttachment(result.knowledge_item_id, attachment.id, attachment.file_name)}>下载 {attachment.file_name}</button></li>)}</ul>}</details>
            {renderKnowledgeImages(result.title, result.images)}
          </article>
        )}</div>}
        {knowledgeAnswer && <div className="m-5 rounded-md border border-slate-200 bg-slate-50 p-4"><p className="m-0 text-xs text-slate-600" role="status">{knowledgeAnswerStatus}</p><p className="mb-0 mt-3 whitespace-pre-wrap text-sm leading-6">{knowledgeAnswer.answer}</p><p className="mb-0 mt-3 text-xs text-slate-600">来源：{knowledgeAnswer.sources.map((source) => `${source.title} v${source.version}`).join("、") || "资料不足"}</p>{knowledgeAnswer.sources.map((source) => <div key={source.knowledge_item_id}>{renderKnowledgeImages(source.title, source.images)}</div>)}<p className="mb-0 mt-3 text-xs text-slate-600">{knowledgeAnswer.human_review_notice}</p></div>}
      </section>
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
                <article key={question.id} id={`learning-question-${question.id}`} className="px-5 py-4">
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
                          answeringQuestion === question.id
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
                      得分 {results[question.id].score}/{results[question.id].maxScore}。{" "}
                      {results[question.id].explanation}
                    </p>
                  )}
                </article>
              ))}
            </div>
          )}
        </section>
        <section className="border-y border-slate-200 bg-white" aria-labelledby="learning-progress-title">
          <header className="border-b border-slate-200 px-5 py-4"><h2 id="learning-progress-title" className="m-0 text-base font-bold">学习记录</h2></header>
          <div className="px-5 py-4 text-sm text-slate-700">
            <p className="m-0">已答 {progress?.answered_questions ?? 0} 题 · 正确 {progress?.correct_answers ?? 0} 题 · 正确率 {Math.round((progress?.accuracy ?? 0) * 100)}%</p>
            <p className="mb-0 mt-2">错题 {wrongAnswers.length} 道 · 历史作答 {answerHistory.length} 次</p>
            {(recommendedQuestion || recommendedKnowledge) && <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-slate-100 pt-3"><span className="text-xs font-semibold text-slate-600">下一步</span>{recommendedKnowledge && <Button size="sm" variant="secondary" onPress={() => { setKnowledgeResults([recommendedKnowledge]); document.getElementById("knowledge-search-title")?.scrollIntoView({ behavior: "smooth" }); }}>阅读 {recommendedKnowledge.title}</Button>}{recommendedQuestion && <Button size="sm" variant="secondary" onPress={jumpToQuestion}>练习 {questions.find((question) => question.id === recommendedQuestion)?.prompt ?? "待练习题目"}</Button>}</div>}
            {wrongAnswers.length > 0 && <ul className="mt-3 list-disc pl-5">{wrongAnswers.map((answer) => <li key={answer.id}>{answer.question_snapshot?.prompt ?? answer.question_id}（v{answer.question_version}）</li>)}</ul>}
            {answerHistory.length > 0 && <details className="mt-3"><summary className="cursor-pointer">查看历史作答</summary><ul className="mt-2 space-y-2">{answerHistory.map((answer) => <li key={answer.id}>{answer.question_snapshot?.prompt ?? answer.question_id} · {answer.is_correct ? "正确" : "错误"} · {formatDate(answer.created_at)}</li>)}</ul></details>}
          </div>
        </section>
        <section className="border-y border-slate-200 bg-white" aria-labelledby="practice-help-title">
          <header className="border-b border-slate-200 px-5 py-4">
            <h2 id="practice-help-title" className="m-0 text-base font-bold text-slate-950">
              练习说明
            </h2>
          </header>
          <div className="px-5 py-6 text-sm leading-6 text-slate-600">
            <p className="m-0">提交前不会显示答案或解析。完成作答后可查看解析与关联的已发布资料。</p>
            <p className="mb-0 mt-3">练习可以重复进行；学习进度以当前题目版本的最新一次结果为准。</p>
          </div>
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
