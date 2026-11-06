import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { KnowledgeImportPreview } from "./KnowledgeImportPreview";
import type { KnowledgeImportBatch } from "../api/learning";

const api = vi.hoisted(() => ({
  preview: vi.fn(),
  confirm: vi.fn(),
  cancel: vi.fn(),
}));
vi.mock("../api/learning", () => ({
  previewKnowledgeImport: api.preview,
  confirmKnowledgeImport: api.confirm,
  cancelKnowledgeImport: api.cancel,
}));
const batch: KnowledgeImportBatch = {
  id: "batch",
  knowledge_base_id: "base",
  file_name: "source.csv",
  source_metadata: {},
  total_rows: 21,
  valid_rows: 21,
  invalid_rows: 0,
  status: "previewed",
  confirmed_at: null,
  rows: Array.from({ length: 21 }, (_, index) => ({
    id: String(index),
    row_number: index + 2,
    status: "valid",
    error_message: null,
    normalized_data: {},
    knowledge_item_id: null,
    raw_data: [
      "base",
      `Title ${index}`,
      "Guidance",
      "",
      "Safety",
      "tag",
      "Source",
      "",
      "learner",
      `EXP-${index}`,
      "{}",
    ],
  })),
};
beforeEach(() => {
  vi.clearAllMocks();
  api.cancel.mockResolvedValue({});
});

it("retains temporary edits across pages and revalidates the edited file before confirmation", async () => {
  const changed = vi.fn();
  api.preview.mockResolvedValue({ ...batch, id: "replacement" });
  render(
    <KnowledgeImportPreview
      token="token"
      batch={batch}
      onChange={changed}
      onImported={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByLabelText("第 2 行 推荐动作"), {
    target: { value: "Assign a mentor" },
  });
  expect(screen.getByRole("button", { name: "确认导入并发布" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "下一页" }));
  expect(screen.getByLabelText("第 22 行 经验标题")).toHaveValue("Title 20");
  fireEvent.click(screen.getByRole("button", { name: "上一页" }));
  expect(screen.getByLabelText("第 2 行 推荐动作")).toHaveValue(
    "Assign a mentor",
  );
  fireEvent.click(screen.getByRole("button", { name: "重新校验修改" }));
  await waitFor(() =>
    expect(changed).toHaveBeenCalledWith(
      expect.objectContaining({ id: "replacement" }),
    ),
  );
  expect(api.preview.mock.calls[0][2]).toBeInstanceOf(File);
  const text = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = reject;
    reader.readAsText(api.preview.mock.calls[0][2]);
  });
  expect(text).toContain("Assign a mentor");
  expect(api.confirm).not.toHaveBeenCalled();
  expect(api.cancel).toHaveBeenCalledWith("token", "batch");
});

it("does not discard edits when revalidation fails and permits a retry", async () => {
  api.preview
    .mockRejectedValueOnce(new Error("Temporary network error"))
    .mockResolvedValueOnce({ ...batch, id: "retry" });
  const changed = vi.fn();
  render(
    <KnowledgeImportPreview
      token="token"
      batch={batch}
      onChange={changed}
      onImported={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByLabelText("第 2 行 原始回答"), {
    target: { value: "Preserved answer" },
  });
  fireEvent.click(screen.getByRole("button", { name: "重新校验修改" }));
  await screen.findByRole("alert");
  expect(screen.getByLabelText("第 2 行 原始回答")).toHaveValue(
    "Preserved answer",
  );
  fireEvent.click(screen.getByRole("button", { name: "重新校验修改" }));
  await waitFor(() =>
    expect(changed).toHaveBeenCalledWith(
      expect.objectContaining({ id: "retry" }),
    ),
  );
});

it("confirms a valid batch once and refreshes the published library", async () => {
  api.confirm.mockResolvedValue({ ...batch, status: "confirmed" });
  const imported = vi.fn();
  render(
    <KnowledgeImportPreview
      token="token"
      batch={batch}
      onChange={vi.fn()}
      onImported={imported}
      onClose={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "确认导入并发布" }));
  await waitFor(() => expect(imported).toHaveBeenCalledOnce());
  expect(api.confirm).toHaveBeenCalledExactlyOnceWith("token", "batch");
});

it("allows correcting original CSV columns on invalid rows before revalidation", async () => {
  const invalid = {
    ...batch,
    total_rows: 1,
    valid_rows: 0,
    invalid_rows: 1,
    rows: [
      {
        ...batch.rows[0],
        status: "invalid",
        error_message:
          "knowledge_base_id does not match the target knowledge base",
        raw_data: ["wrong-base", ...batch.rows[0].raw_data!.slice(1)],
      },
    ],
  };
  api.preview.mockResolvedValue({ ...invalid, id: "corrected" });
  render(
    <KnowledgeImportPreview
      token="token"
      batch={invalid}
      onChange={vi.fn()}
      onImported={vi.fn()}
      onClose={vi.fn()}
    />,
  );
  expect(screen.getByLabelText("第 2 行 知识库ID")).toHaveValue("wrong-base");
  fireEvent.change(screen.getByLabelText("第 2 行 知识库ID"), {
    target: { value: "base" },
  });
  fireEvent.change(screen.getByLabelText("第 2 行 可见范围"), {
    target: { value: "admin" },
  });
  expect(screen.getByRole("button", { name: "确认导入并发布" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "重新校验修改" }));
  await waitFor(() => expect(api.preview).toHaveBeenCalledOnce());
  const contents = await new Promise<string>((resolve) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.readAsText(api.preview.mock.calls[0][2]);
  });
  expect(contents).toContain('"base","Title 0"');
  expect(contents).toContain('"admin","EXP-0"');
  expect(contents).not.toContain("wrong-base");
});
