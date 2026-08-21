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
  downloadCaseAttachment: vi.fn(),
  updateElderProfile: vi.fn(),
  uploadCaseAttachment: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({ useAuth: () => mocked.auth }));
vi.mock("../api/cases", () => ({
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  getCasePublicProgress: (...args: unknown[]) => mocked.getCasePublicProgress(...args),
  getCaseResourceConfiguration: (...args: unknown[]) => mocked.getCaseResourceConfiguration(...args),
  createClue: (...args: unknown[]) => mocked.createClue(...args),
  createCasePlace: (...args: unknown[]) => mocked.createCasePlace(...args),
  downloadCaseAttachment: (...args: unknown[]) => mocked.downloadCaseAttachment(...args),
  updateElderProfile: (...args: unknown[]) => mocked.updateElderProfile(...args),
  uploadCaseAttachment: (...args: unknown[]) => mocked.uploadCaseAttachment(...args),
}));

const detail: CaseDetail = {
  id: "case-1", case_code: "AG-CASE-1", status: "active", access_role: "family",
  elder_profile: { id: "profile-1", display_name: "王奶奶", age: 82, gender: "女", physical_description: "戴眼镜", clothing_description: "蓝外套", health_notes: null, last_seen_at: "2026-08-10T00:00:00Z", last_seen_location: "南门", mobility_notes: null, transportation_ability: null, frequent_locations: null, behavior_habits: null, suspicious_motive: null },
  clues: [], places: [{ id: "place-1", case_id: "case-1", name: "社区公园", place_type: "frequent", address: "社区公园", longitude: null, latitude: null, source: "family", visibility: "confirmed", review_status: "confirmed", created_at: "2026-08-10T00:00:00Z", updated_at: "2026-08-10T00:00:00Z", is_own_submission: true }], attachments: [], created_at: "2026-08-10T00:00:00Z", updated_at: "2026-08-10T00:00:00Z",
};

const detailWithAttachment: CaseDetail = {
  ...detail,
  attachments: [
    {
      id: "attachment-1",
      case_id: "case-1",
      original_filename: "family-photo.png",
      content_type: "image/png",
      byte_size: 32,
      source: "family",
      review_status: "pending_review",
      created_at: "2026-08-10T00:00:00Z",
      updated_at: "2026-08-10T00:00:00Z",
      is_own_submission: true,
    },
  ],
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
    mocked.updateElderProfile.mockResolvedValue(detail);
  });

  it("shows reviewed public progress without leaking internal clue content", async () => {
    renderPage();
    expect(await screen.findByRole("heading", { name: "公开进展" })).toBeInTheDocument();
    expect(screen.getByText("已审核的进展更新")).toBeInTheDocument();
    expect(screen.getByText(/提交的资料、地点和线索会先进入人工审核/)).toBeInTheDocument();
    expect(screen.queryByText("secret internal clue")).not.toBeInTheDocument();
  });

  it("submits a supplementary family clue through the safe public action", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "公开进展" });
    fireEvent.click(screen.getByRole("button", { name: /提交一条新线索/ }));
    fireEvent.change(screen.getByRole("textbox", { name: "新线索内容" }), { target: { value: "在社区公园附近看到相似衣着" } });
    fireEvent.click(screen.getByRole("button", { name: "提交线索" }));
    await waitFor(() => expect(mocked.createClue).toHaveBeenCalledWith("family-token", "case-1", expect.objectContaining({ source: "family", source_type: "manual_report", content: "在社区公园附近看到相似衣着", location_precision: null })));
  });

  it("saves corrected family profile fields without exposing operational controls", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "公开进展" });
    fireEvent.click(screen.getByRole("button", { name: /补充或更正人物资料/ }));
    fireEvent.change(screen.getByLabelText("年龄"), { target: { value: "83" } });
    fireEvent.change(screen.getByLabelText("体貌"), { target: { value: "佩戴深色眼镜" } });
    fireEvent.click(screen.getByRole("button", { name: "保存人物摘要" }));

    await waitFor(() =>
      expect(mocked.updateElderProfile).toHaveBeenCalledWith(
        "family-token",
        "case-1",
        expect.objectContaining({ age: 83, physical_description: "佩戴深色眼镜" }),
      ),
    );
  });

  it("downloads a family-owned attachment into a local preview", async () => {
    mocked.getCase.mockResolvedValue(detailWithAttachment);
    mocked.downloadCaseAttachment.mockResolvedValue(
      new Blob(["private preview"], { type: "image/png" }),
    );
    const createObjectUrl = vi.fn(() => "blob:private-case-photo");
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: createObjectUrl,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: vi.fn(),
    });

    renderPage();
    expect(await screen.findByRole("img", { name: "family-photo.png 预览" })).toHaveAttribute(
      "src",
      "blob:private-case-photo",
    );
    expect(mocked.downloadCaseAttachment).toHaveBeenCalledWith(
      "family-token",
      "case-1",
      "attachment-1",
    );
  });

  it("submits a place with selected coordinates", async () => {
    mocked.createCasePlace.mockResolvedValue({});
    renderPage();
    await screen.findByRole("heading", { name: "常去地点" });
    expect(screen.getByLabelText("经度")).toHaveAttribute("type", "text");
    expect(screen.getByLabelText("纬度")).toHaveAttribute("inputmode", "decimal");
    fireEvent.change(screen.getByLabelText("地点名称"), { target: { value: "社区花园" } });
    fireEvent.change(screen.getByLabelText("文字地址"), { target: { value: "虹桥路 100 号" } });
    fireEvent.change(screen.getByLabelText("经度"), { target: { value: "121.41" } });
    fireEvent.change(screen.getByLabelText("纬度"), { target: { value: "31.21" } });
    fireEvent.submit(screen.getByRole("button", { name: "提交地点" }).closest("form")!);

    await waitFor(() =>
      expect(mocked.createCasePlace).toHaveBeenCalledWith(
        "family-token",
        "case-1",
        expect.objectContaining({
          name: "社区花园",
          address: "虹桥路 100 号",
          longitude: 121.41,
          latitude: 31.21,
        }),
      ),
    );
  });

  it("keeps incomplete coordinate input visible and rejects it on submit", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "常去地点" });
    fireEvent.change(screen.getByLabelText("地点名称"), { target: { value: "社区花园" } });
    fireEvent.change(screen.getByLabelText("文字地址"), { target: { value: "虹桥路 100 号" } });
    fireEvent.change(screen.getByLabelText("经度"), { target: { value: "-" } });
    expect(screen.getByLabelText("经度")).toHaveValue("-");
    fireEvent.change(screen.getByLabelText("经度"), { target: { value: "121." } });
    expect(screen.getByLabelText("经度")).toHaveValue("121.");

    fireEvent.change(screen.getByLabelText("经度"), { target: { value: "-" } });
    fireEvent.change(screen.getByLabelText("纬度"), { target: { value: "31.21" } });
    fireEvent.submit(screen.getByRole("button", { name: "提交地点" }).closest("form")!);
    expect(await screen.findByRole("alert")).toHaveTextContent("经度和纬度必须是有效数字。");
    expect(mocked.createCasePlace).not.toHaveBeenCalled();
  });
});
