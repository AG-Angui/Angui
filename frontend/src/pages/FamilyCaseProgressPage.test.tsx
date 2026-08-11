import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CaseDetail } from "../api/cases";
import { FamilyCaseProgressPage } from "./FamilyCaseProgressPage";

const mocked = vi.hoisted(() => ({
  auth: { token: "family-token" },
  getCase: vi.fn(),
  getCasePublicProgress: vi.fn(),
  getCaseResourceConfiguration: vi.fn(),
  createClue: vi.fn(),
  createCasePlace: vi.fn(),
  uploadCaseAttachment: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({ useAuth: () => mocked.auth }));
vi.mock("../api/cases", () => ({
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  getCasePublicProgress: (...args: unknown[]) => mocked.getCasePublicProgress(...args),
  getCaseResourceConfiguration: (...args: unknown[]) => mocked.getCaseResourceConfiguration(...args),
  createClue: (...args: unknown[]) => mocked.createClue(...args),
  createCasePlace: (...args: unknown[]) => mocked.createCasePlace(...args),
  uploadCaseAttachment: (...args: unknown[]) => mocked.uploadCaseAttachment(...args),
}));

const detail: CaseDetail = {
  id: "case-1", case_code: "AG-CASE-1", status: "active", access_role: "family",
  elder_profile: { id: "profile-1", display_name: "王奶奶", age: 82, gender: "女", physical_description: "戴眼镜", clothing_description: "蓝外套", health_notes: null, last_seen_at: "2026-08-10T00:00:00Z", last_seen_location: "南门" },
  clues: [], places: [{ id: "place-1", case_id: "case-1", name: "社区公园", place_type: "frequent", address: "社区公园", longitude: null, latitude: null, source: "family", visibility: "confirmed", review_status: "confirmed", created_at: "2026-08-10T00:00:00Z", updated_at: "2026-08-10T00:00:00Z", is_own_submission: true }], attachments: [], created_at: "2026-08-10T00:00:00Z", updated_at: "2026-08-10T00:00:00Z",
};

function renderPage() {
  return render(<MemoryRouter initialEntries={["/family/cases/case-1"]}><Routes><Route path="/family/cases/:caseId" element={<FamilyCaseProgressPage />} /></Routes></MemoryRouter>);
}

describe("FamilyCaseProgressPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.getCase.mockResolvedValue(detail);
    mocked.getCasePublicProgress.mockResolvedValue({ case_id: "case-1", status: "active", publication_status: "reviewed_public", generated_at: "2026-08-10T00:00:00Z", confirmed_progress: [{ clue_id: "clue-public", progress_type: "confirmed_update", review_status: "confirmed", updated_at: "2026-08-10T00:00:00Z" }], requested_family_information: [], safety_and_contact_reminders: ["如有紧急情况请联系当地警方。"] });
    mocked.getCaseResourceConfiguration.mockResolvedValue({ attachment_max_image_bytes: 5 * 1024 * 1024, attachment_max_per_case: 10, case_place_types: ["frequent"] });
    mocked.createClue.mockResolvedValue({});
  });

  it("shows reviewed public progress without leaking internal clue content", async () => {
    renderPage();
    expect(await screen.findByRole("heading", { name: "公开进展" })).toBeInTheDocument();
    expect(screen.getByText("已审核的进展更新")).toBeInTheDocument();
    expect(screen.getByText(/内部调度和未核实线索不会在家属端显示/)).toBeInTheDocument();
    expect(screen.queryByText("secret internal clue")).not.toBeInTheDocument();
  });

  it("submits a supplementary family clue through the safe public action", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "公开进展" });
    fireEvent.change(screen.getByRole("textbox", { name: "新线索内容" }), { target: { value: "在社区公园附近看到相似衣着" } });
    fireEvent.click(screen.getByRole("button", { name: "提交线索" }));
    await waitFor(() => expect(mocked.createClue).toHaveBeenCalledWith("family-token", "case-1", expect.objectContaining({ source: "family", source_type: "manual_report", content: "在社区公园附近看到相似衣着", location_precision: null })));
  });
});
