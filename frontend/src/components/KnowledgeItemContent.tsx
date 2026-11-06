import type { presentKnowledgeItem } from "../utils/knowledgePresentation";

export function KnowledgeItemContent({
  presentation,
}: {
  presentation: ReturnType<typeof presentKnowledgeItem>;
}) {
  const { body, sourceDetails } = presentation;
  return (
    <div className="min-w-0">
      <p className="m-0 line-clamp-4 whitespace-pre-wrap break-words">
        {body}
      </p>
      {(body.length > 160 || body.includes("\n") || sourceDetails.length > 0) && (
        <details className="mt-2 text-xs">
          <summary className="cursor-pointer text-brand-700 focus-visible:outline-2 focus-visible:outline-brand-600">
            查看完整正文与原始资料
          </summary>
          <div className="mt-2 max-h-96 space-y-4 overflow-y-auto rounded-md border border-slate-200 bg-slate-50 p-3">
            <p className="m-0 whitespace-pre-wrap break-words text-sm">{body}</p>
            {sourceDetails.length > 0 && (
              <dl className="m-0 space-y-3 border-t border-slate-200 pt-3">
                {sourceDetails.map(([key, value]) => (
                  <div key={key}>
                    <dt className="font-semibold text-slate-900">{key}</dt>
                    <dd className="m-0 mt-1 whitespace-pre-wrap break-words">
                      {typeof value === "string" ? value : JSON.stringify(value)}
                    </dd>
                  </div>
                ))}
              </dl>
            )}
          </div>
        </details>
      )}
    </div>
  );
}
