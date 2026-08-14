import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FamilyHomePage } from "./FamilyHomePage";

const mocked = vi.hoisted(() => ({ listCases: vi.fn(), token: "family-token" }));

vi.mock("../auth/useAuth", () => ({ useAuth: () => ({ token: mocked.token }) }));
vi.mock("../api/cases", () => ({ listCases: (...args: unknown[]) => mocked.listCases(...args) }));

function renderPage() {
  return render(<MemoryRouter><FamilyHomePage /></MemoryRouter>);
}

describe("FamilyHomePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("gives a family without a case a clear request entry", async () => {
    mocked.listCases.mockResolvedValue([]);
    renderPage();
    expect(await screen.findByRole("heading", { name: "说清楚情况，安心看进展" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /开始求助/ })).toHaveAttribute("href", "/family/intake");
    expect(screen.getByText("还没有求助记录。开始填写后，离开页面也可以从草稿继续。")).toBeInTheDocument();
    await waitFor(() => expect(mocked.listCases).toHaveBeenCalledWith("family-token"));
  });

  it("shows only family cases and leads to public progress", async () => {
    mocked.listCases.mockResolvedValue([
      { id: "family-case", case_code: "AG-FAMILY", status: "active", access_role: "family", display_name: "李阿姨", last_seen_at: null, last_seen_location: "南门", created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" },
      { id: "internal-case", case_code: "AG-INTERNAL", status: "active", access_role: "commander", display_name: "不应显示", last_seen_at: null, last_seen_location: null, created_at: "2026-08-01T00:00:00Z", updated_at: "2026-08-01T00:00:00Z" },
    ]);
    renderPage();
    expect(await screen.findByRole("heading", { name: "李阿姨" })).toBeInTheDocument();
    expect(screen.queryByText("不应显示")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: /查看公开进展/ })).toHaveAttribute("href", "/family/cases/family-case");
  });
});
