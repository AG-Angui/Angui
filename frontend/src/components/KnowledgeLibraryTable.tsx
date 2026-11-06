import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  listKnowledgeItemsPage,
  type KnowledgeItem,
  type KnowledgeItemPage,
} from "../api/learning";
import { experienceFields, knowledgeFields } from "../utils/knowledgeFields";
import { KnowledgePagination } from "./KnowledgePagination";

export function KnowledgeLibraryTable({
  token,
  baseId,
  refresh,
  onItems,
  renderActions,
}: {
  token: string;
  baseId: string;
  refresh: number;
  onItems: (items: KnowledgeItem[]) => void;
  renderActions: (item: KnowledgeItem) => ReactNode;
}) {
  const [query, setQuery] = useState({ page: 1, size: 20 });
  const [data, setData] = useState<KnowledgeItemPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);
  const onItemsRef = useRef(onItems);
  onItemsRef.current = onItems;
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    setData(null);
    onItemsRef.current([]);
    void listKnowledgeItemsPage(token, baseId, query.page, query.size)
      .then((result) => {
        if (!active) return;
        setData(result);
        onItemsRef.current(result.items);
      })
      .catch((cause) => {
        if (active)
          setError(cause instanceof Error ? cause.message : "加载失败");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [token, baseId, query.page, query.size, refresh, retry]);
  const page = data?.page ?? query.page;
  return (
    <section className="mt-5" aria-label="知识库条目">
      <KnowledgePagination
        label="知识库分页"
        page={page}
        size={query.size}
        total={data?.total ?? 0}
        disabled={loading}
        onChange={(page, size) => setQuery({ page, size })}
      />
      {error && (
        <p role="alert" className="text-rose-700">
          {error}{" "}
          <button
            type="button"
            className="underline"
            onClick={() => setRetry((value) => value + 1)}
          >
            重新加载
          </button>
        </p>
      )}
      {loading && (
        <p role="status" className="p-3">
          正在加载本页条目…
        </p>
      )}
      <div
        className={
          query.size > 20 ? "max-h-[640px] overflow-auto" : "overflow-x-auto"
        }
        aria-busy={loading}
      >
        <table
          style={{ width: 128 + experienceFields.length * 256 }}
          className="min-w-full table-fixed text-left text-sm"
        >
          <caption className="sr-only">Knowledge items</caption>
          <thead className="sticky top-0 z-10 bg-slate-50 text-xs text-slate-600">
            <tr>
              {experienceFields.map((field) => (
                <th
                  scope="col"
                  key={field}
                  className={
                    field === "经验ID"
                      ? "w-32 min-w-32 border-b px-3 py-3"
                      : "w-64 min-w-64 border-b px-3 py-3"
                  }
                >
                  {field === "经验标题" ? "标题" : field}
                </th>
              ))}
              <th scope="col" className="w-64 min-w-64 border-b p-3">
                图片 / 附件 / 操作
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {data?.items.map((item) => {
              const fields = knowledgeFields(item);
              return (
                <tr key={item.knowledge_item_id} className="align-top">
                  {experienceFields.map((field) => (
                    <td key={field} className="p-3 text-slate-700">
                      <details className="group">
                        <summary
                          title={field === "经验标题" ? item.title : undefined}
                          className="line-clamp-4 cursor-pointer list-none whitespace-pre-wrap break-words group-open:line-clamp-none"
                        >
                          {fields[field] || "—"}
                        </summary>
                      </details>
                    </td>
                  ))}
                  <td className="p-3">{renderActions(item)}</td>
                </tr>
              );
            })}
            {!loading && !error && data?.items.length === 0 && (
              <tr>
                <td
                  colSpan={experienceFields.length + 1}
                  className="p-6 text-center text-slate-500"
                >
                  暂无条目
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <p className="mt-2 text-xs text-slate-500">
        左右滚动查看独立字段；点击长文本可展开。每页超过 20
        条时在表格内上下滚动。
      </p>
    </section>
  );
}
