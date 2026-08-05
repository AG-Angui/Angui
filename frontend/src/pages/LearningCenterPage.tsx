import { Button, Chip, Input, Spinner } from "@heroui/react";
import { BookOpen, Send, WifiOff } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  askKnowledge,
  getPublicPreventionCard,
  listLearningQuestions,
  listLearningResources,
  submitLearningAnswer,
} from "../api/learning";
import type {
  KnowledgeAnswer,
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

export function LearningCenterPage() {
  const { token } = useAuth();
  const preventionCacheReady = usePreventionCacheReady();
  const [resources, setResources] = useState<LearningResource[]>([]);
  const [questions, setQuestions] = useState<LearningQuestion[]>([]);
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
        listLearningResources(token),
        listLearningQuestions(token),
      ]);
      setResources(nextResources);
      setQuestions(nextQuestions);
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
  }, [token]);

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
