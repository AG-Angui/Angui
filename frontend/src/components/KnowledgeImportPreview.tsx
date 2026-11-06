import { useRef, useState } from "react";
import {
  cancelKnowledgeImport,
  confirmKnowledgeImport,
  previewKnowledgeImport,
  type KnowledgeImportBatch,
} from "../api/learning";
import {
  csvHeaders,
  csvRowItem,
  editCsvField,
  experienceFields,
  knowledgeFields,
  serializeKnowledgeCsv,
} from "../utils/knowledgeFields";
import { KnowledgePagination } from "./KnowledgePagination";

const rawColumns = [
  { index: 0, label: "知识库ID" },
  { index: 2, label: "正文" },
  { index: 3, label: "摘要" },
  { index: 5, label: "关键词" },
  { index: 7, label: "来源链接" },
  { index: 8, label: "可见范围" },
  { index: 10, label: "元数据 JSON" },
];

export function KnowledgeImportPreview({
  token,
  batch,
  onChange,
  onImported,
  onClose,
}: {
  token: string;
  batch: KnowledgeImportBatch;
  onChange: (batch: KnowledgeImportBatch) => void;
  onImported: () => void;
  onClose: () => void;
}) {
  const [rows, setRows] = useState(() =>
    batch.rows.map((row) => {
      if (row.raw_data)
        return Array.from(
          { length: 11 },
          (_, index) => row.raw_data![index] || "",
        );
      const data = row.normalized_data;
      return csvHeaders.map((key) =>
        key === "metadata"
          ? JSON.stringify(data[key] ?? {})
          : key === "keywords"
            ? ((data[key] as string[]) || []).join(",")
            : String(data[key] ?? ""),
      );
    }),
  );
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(20);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const [error, setError] = useState("");
  const editable = batch.status === "previewed";
  const run = async (action: "validate" | "confirm" | "cancel") => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setError("");
    try {
      if (action === "validate") {
        const file = new File([serializeKnowledgeCsv(rows)], batch.file_name, {
          type: "text/csv",
        });
        const next = await previewKnowledgeImport(
          token,
          batch.knowledge_base_id,
          file,
          batch.source_metadata,
        );
        onChange(next);
        // Replacing the preview must never confirm the previous batch.
        await cancelKnowledgeImport(token, batch.id).catch(() => undefined);
      } else if (action === "confirm") {
        if (dirty || batch.invalid_rows > 0 || batch.valid_rows === 0)
          throw new Error("请先修正错误并重新校验，再确认导入。");
        onChange(await confirmKnowledgeImport(token, batch.id));
        onImported();
      } else {
        await cancelKnowledgeImport(token, batch.id);
        onClose();
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "导入失败，请重试");
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  };
  return (
    <section
      className="border-t border-slate-200 p-5"
      aria-label="CSV 内容预览"
    >
      <h3 className="m-0 text-sm font-semibold">
        {batch.status === "confirmed" ? "已导入并发布" : "CSV 内容预览"}：
        {batch.file_name}
      </h3>
      <p className="text-xs text-slate-600">
        共 {rows.length} 条，{batch.valid_rows} 条有效，{batch.invalid_rows}{" "}
        条待处理。临时修改仅在重新校验并确认导入后保存，不会改动原 CSV 文件。
      </p>
      {error && (
        <p role="alert" className="text-sm text-rose-700">
          {error}
        </p>
      )}
      {dirty && (
        <p role="status" className="text-sm text-amber-700">
          有未校验的修改，请重新校验。
        </p>
      )}
      <KnowledgePagination
        label="CSV 预览分页"
        page={page}
        size={size}
        total={rows.length}
        disabled={busy}
        onChange={(page, size) => {
          setPage(page);
          setSize(size);
        }}
      />
      <div className="max-h-[640px] overflow-auto">
        <table className="w-max min-w-full text-left text-xs">
          <thead className="sticky top-0 z-10 bg-slate-50">
            <tr>
              <th className="min-w-40 p-2">行 / 校验结果</th>
              {experienceFields.map((field) => (
                <th key={field} className="min-w-56 p-2">
                  {field}
                </th>
              ))}
              {rawColumns.map(({ index, label }) => (
                <th className="min-w-56 p-2" key={index}>
                  {label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows
              .slice((page - 1) * size, page * size)
              .map((cells, localIndex) => {
                const index = (page - 1) * size + localIndex;
                const result = batch.rows[index];
                const fields = knowledgeFields(csvRowItem(cells), false);
                const update = (next: string[]) => {
                  setRows((current) =>
                    current.map((row, rowIndex) =>
                      rowIndex === index ? next : row,
                    ),
                  );
                  setDirty(true);
                };
                return (
                  <tr key={index} className="border-t align-top">
                    <td className="p-2">
                      第 {result.row_number} 行<br />
                      {dirty ? "待校验" : result.status}
                      <p className="text-rose-700">
                        {dirty ? "" : result.error_message}
                      </p>
                    </td>
                    {experienceFields.map((field) => (
                      <td className="p-2" key={field}>
                        <textarea
                          aria-label={
                            "第 " + result.row_number + " 行 " + field
                          }
                          className="min-h-24 w-56 rounded border border-slate-200 p-2"
                          disabled={!editable || busy}
                          value={
                            field === "经验标题"
                              ? cells[1]
                              : fields[field] || ""
                          }
                          onChange={(event) =>
                            update(
                              editCsvField(cells, field, event.target.value),
                            )
                          }
                        />
                      </td>
                    ))}
                    {rawColumns.map(({ index: column, label }) => (
                      <td className="p-2" key={column}>
                        <textarea
                          aria-label={
                            "第 " + result.row_number + " 行 " + label
                          }
                          className="min-h-24 w-56 rounded border border-slate-200 p-2"
                          disabled={!editable || busy}
                          value={cells[column] || ""}
                          onChange={(event) => {
                            const next = [...cells];
                            next[column] = event.target.value;
                            update(next);
                          }}
                        />
                      </td>
                    ))}
                  </tr>
                );
              })}
          </tbody>
        </table>
      </div>
      {editable ? (
        <div className="mt-3 flex flex-wrap gap-3">
          <button
            type="button"
            className="rounded border px-4 py-2 disabled:opacity-40"
            disabled={busy}
            onClick={() => void run("validate")}
          >
            重新校验修改
          </button>
          <button
            type="button"
            className="rounded bg-brand-700 px-4 py-2 text-white disabled:opacity-40"
            disabled={
              busy || dirty || batch.invalid_rows > 0 || batch.valid_rows === 0
            }
            onClick={() => void run("confirm")}
          >
            确认导入并发布
          </button>
          <button
            type="button"
            className="rounded border px-4 py-2 disabled:opacity-40"
            disabled={busy}
            onClick={() => void run("cancel")}
          >
            取消导入
          </button>
        </div>
      ) : (
        <button type="button" className="mt-3 underline" onClick={onClose}>
          关闭预览
        </button>
      )}
    </section>
  );
}
