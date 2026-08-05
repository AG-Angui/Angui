export type AiReviewStage =
  | "queued"
  | "preparing"
  | "generating"
  | "validating"
  | "fallback"
  | "ready_for_review"
  | "failed";

const stageDetails: Record<
  AiReviewStage,
  { label: string; description: string; completed: number }
> = {
  queued: {
    label: "已提交审核请求",
    description: "正在安排受控审核，不会创建正式记录。",
    completed: 1,
  },
  preparing: {
    label: "正在准备受控资料",
    description: "系统正在核对当前授权范围并最小化处理输入。",
    completed: 2,
  },
  generating: {
    label: "正在生成审核候选",
    description: "生成的内容仍是草稿，尚未成为已确认事实。",
    completed: 3,
  },
  validating: {
    label: "正在校验审核候选",
    description: "系统正在核对结构、来源范围和业务约束。",
    completed: 4,
  },
  fallback: {
    label: "正在切换规则结果",
    description: "AI 结果不可用或未通过校验，系统会提供可人工核对的规则结果。",
    completed: 4,
  },
  ready_for_review: {
    label: "候选已准备好，等待人工审核",
    description: "结果仍是草稿或候选，必须由当前角色人工核对后才能继续。",
    completed: 4,
  },
  failed: {
    label: "审核未能完成",
    description: "没有生成可用草稿。请重试，或按现有人工流程继续。",
    completed: 0,
  },
};

export function AiReviewProgress({
  stage,
  title = "AI 审核进行中",
}: {
  stage: AiReviewStage;
  title?: string;
}) {
  const detail = stageDetails[stage];

  return (
    <section
      className="mt-4 rounded-md border border-brand-200 bg-brand-50 p-4 text-sm text-slate-700"
      role="status"
      aria-live="polite"
      aria-atomic="true"
      aria-label={`${title}：${detail.label}`}
    >
      <div className="flex items-start gap-3">
        <span
          className="mt-1 flex h-3 w-3 shrink-0 rounded-full bg-brand-600 motion-safe:animate-pulse motion-reduce:animate-none"
          aria-hidden="true"
        />
        <div className="min-w-0 flex-1">
          <h3 className="m-0 text-sm font-bold text-slate-950">{title}</h3>
          <p className="mb-0 mt-1 leading-6">{detail.label}</p>
          <p className="mb-0 mt-1 text-xs leading-5 text-slate-600">
            {detail.description}
          </p>
        </div>
      </div>
      <ol className="mb-0 mt-4 grid grid-cols-4 gap-2 p-0" aria-label="审核进度">
        {["已提交", "准备资料", "生成候选", "校验结果"].map((label, index) => {
          const isComplete = index < detail.completed;
          const isCurrent = index + 1 === detail.completed;
          return (
            <li key={label} className="list-none">
              <span
                className={`block h-1.5 rounded-full ${
                  isComplete ? "bg-brand-600" : "bg-brand-100"
                } ${
                  isCurrent
                    ? "motion-safe:animate-pulse motion-reduce:animate-none"
                    : ""
                }`}
                aria-hidden="true"
              />
              <span className="mt-1 block text-[11px] leading-4 text-slate-600">
                {label}
              </span>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
