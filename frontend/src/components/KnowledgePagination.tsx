export function KnowledgePagination({
  page,
  size,
  total,
  disabled,
  onChange,
  label,
}: {
  page: number;
  size: number;
  total: number;
  disabled?: boolean;
  onChange: (page: number, size: number) => void;
  label: string;
}) {
  const pages = Math.max(1, Math.ceil(total / size));
  return (
    <nav
      aria-label={label}
      className="flex flex-wrap items-center gap-3 border-y border-slate-200 bg-white px-3 py-3 text-sm"
    >
      <span>
        共 {total} 条 · 第 {page} / {pages} 页
      </span>
      <label className="flex items-center gap-2">
        每页
        <select
          aria-label={label + "每页条数"}
          className="rounded border border-slate-300 bg-white p-1"
          value={size}
          disabled={disabled}
          onChange={(event) => onChange(1, Number(event.target.value))}
        >
          {[10, 20, 50, 100].map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
      </label>
      <button
        type="button"
        className="rounded border px-3 py-1 disabled:opacity-40"
        disabled={disabled || page <= 1}
        onClick={() => onChange(page - 1, size)}
      >
        上一页
      </button>
      <button
        type="button"
        className="rounded border px-3 py-1 disabled:opacity-40"
        disabled={disabled || page >= pages}
        onClick={() => onChange(page + 1, size)}
      >
        下一页
      </button>
    </nav>
  );
}
