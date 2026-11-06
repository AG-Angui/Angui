import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { KnowledgeLibraryTable } from "./KnowledgeLibraryTable";
import type { KnowledgeItem } from "../api/learning";

const api = vi.hoisted(() => ({ page: vi.fn() }));
vi.mock("../api/learning", () => ({ listKnowledgeItemsPage: api.page }));
const item: KnowledgeItem = {
  knowledge_item_id: "item-1",
  title: "新人支持：主力带新人",
  content: "【判断结论】\n主力带新人",
  summary: "",
  category: "安全",
  category_id: null,
  keywords: [],
  score: 0,
  knowledge_base_id: "base",
  version: 1,
  source_name: "问卷",
  source_url: null,
  images: [],
  status: "published",
};
beforeEach(() => {
  api.page.mockReset();
  api.page.mockImplementation((_token, _base, page, size) =>
    Promise.resolve({
      items: [{ ...item, knowledge_item_id: `item-${page}` }],
      total: 134,
      page,
      page_size: size,
    }),
  );
});

it("requests only the selected server page and renders independent source columns", async () => {
  render(
    <KnowledgeLibraryTable
      token="token"
      baseId="base"
      refresh={0}
      onItems={vi.fn()}
      renderActions={() => null}
    />,
  );
  await screen.findByText("新人支持");
  expect(api.page).toHaveBeenLastCalledWith("token", "base", 1, 20);
  const table = screen.getByRole("table", { name: "Knowledge items" });
  expect(
    within(table).getByRole("columnheader", { name: "判断结论" }),
  ).toBeInTheDocument();
  expect(
    within(table).queryByRole("columnheader", { name: "状态" }),
  ).not.toBeInTheDocument();
  expect(within(table).queryByText("v1")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "下一页" }));
  await waitFor(() =>
    expect(api.page).toHaveBeenLastCalledWith("token", "base", 2, 20),
  );
  await screen.findByText("新人支持");
  fireEvent.change(screen.getByLabelText("知识库分页每页条数"), {
    target: { value: "50" },
  });
  await waitFor(() =>
    expect(api.page).toHaveBeenLastCalledWith("token", "base", 1, 50),
  );
  expect(table.parentElement).toHaveClass("max-h-[640px]", "overflow-auto");
});

it("allows retrying a failed first request without refreshing the page", async () => {
  api.page.mockRejectedValueOnce(new Error("temporary failure"));
  render(
    <KnowledgeLibraryTable
      token="token"
      baseId="base"
      refresh={0}
      onItems={vi.fn()}
      renderActions={() => null}
    />,
  );
  await screen.findByRole("alert");
  fireEvent.click(screen.getByRole("button", { name: "重新加载" }));
  await screen.findByText("新人支持");
  expect(api.page).toHaveBeenCalledTimes(2);
});
