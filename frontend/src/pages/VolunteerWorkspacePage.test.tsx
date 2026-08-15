import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { CaseDetail, CaseSummary } from "../api/cases";
import { VolunteerWorkspacePage } from "./VolunteerWorkspacePage";

const mocked = vi.hoisted(() => ({
  listCases: vi.fn(),
  listMyTasks: vi.fn(),
  getCase: vi.fn(),
  getCaseSummary: vi.fn(),
  listVolunteerPublishedSummaryVersions: vi.fn(),
  listCaseTasks: vi.fn(),
}));

vi.mock("../auth/useAuth", () => ({
  useAuth: () => ({ token: "test-session" }),
}));

vi.mock("../api/cases", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api/cases")>()),
  listCases: (...args: unknown[]) => mocked.listCases(...args),
  listMyTasks: (...args: unknown[]) => mocked.listMyTasks(...args),
  getCase: (...args: unknown[]) => mocked.getCase(...args),
  getCaseSummary: (...args: unknown[]) => mocked.getCaseSummary(...args),
  listVolunteerPublishedSummaryVersions: (...args: unknown[]) =>
    mocked.listVolunteerPublishedSummaryVersions(...args),
  listCaseTasks: (...args: unknown[]) => mocked.listCaseTasks(...args),
}));

const detail: CaseDetail = {
  id: "case-volunteer",
  case_code: "AG-VOLUNTEER",
  status: "active",
  access_role: "volunteer",
  elder_profile: {
    id: "profile-volunteer",
    display_name: "Test volunteer case",
    age: null,
    gender: null,
    physical_description: null,
    clothing_description: null,
    health_notes: null,
    last_seen_at: null,
    last_seen_location: null,
  },
  clues: [
    {
      id: "clue-confirmed",
      case_id: "case-volunteer",
      status: "confirmed",
      source: "family",
      source_type: "manual_report",
      content: "Confirmed clue content",
      raw_record_reference: null,
      occurred_at: null,
      reported_at: "2026-08-14T08:00:00Z",
      confirmed_at: "2026-08-14T09:00:00Z",
      location_text: "Confirmed clue location",
      location_precision: "approximate",
      next_action: null,
      linked_task_reference: null,
      related_clue_id: null,
      relationship_type: null,
      review_reason: null,
      attachment_ids: [],
      created_at: "2026-08-14T08:00:00Z",
      updated_at: "2026-08-14T09:00:00Z",
      reviewed_at: "2026-08-14T09:00:00Z",
      is_own_submission: false,
    },
  ],
  places: [
    {
      id: "place-family",
      case_id: "case-volunteer",
      name: "Family-approved meeting point",
      place_type: "key_location",
      address: "Family-approved location text",
      longitude: null,
      latitude: null,
      source: "family",
      visibility: "confirmed",
      review_status: "confirmed",
      created_at: "2026-08-14T08:00:00Z",
      updated_at: "2026-08-14T09:00:00Z",
      is_own_submission: false,
    },
  ],
  attachments: [],
  created_at: "2026-08-14T08:00:00Z",
  updated_at: "2026-08-14T09:00:00Z",
};

const summary: CaseSummary = {
  case_id: "case-volunteer",
  access_role: "volunteer",
  generated_at: "2026-08-14T09:00:00Z",
  source_scope: ["confirmed_clues"],
  last_confirmed_information: null,
  confirmed_clues: [],
  pending_verification: [],
  excluded_directions: [],
  current_focus: [],
  task_status: [],
  safety_reminders: [],
};

describe("VolunteerWorkspacePage", () => {
  it("shows published summary history, clue locations, and approved family places", async () => {
    vi.clearAllMocks();
    mocked.listCases.mockResolvedValue([
      {
        id: "case-volunteer",
        case_code: "AG-VOLUNTEER",
        status: "active",
        access_role: "volunteer",
        display_name: "Test volunteer case",
        last_seen_at: null,
        last_seen_location: null,
        created_at: "2026-08-14T08:00:00Z",
        updated_at: "2026-08-14T09:00:00Z",
      },
    ]);
    mocked.listMyTasks.mockResolvedValue([]);
    mocked.getCase.mockResolvedValue(detail);
    mocked.getCaseSummary.mockResolvedValue(summary);
    mocked.listVolunteerPublishedSummaryVersions.mockResolvedValue({
      items: [
        {
          version: 2,
          content: "Published v2 summary",
          published_at: "2026-08-14T09:00:00Z",
        },
        {
          version: 1,
          content: "Published v1 summary",
          published_at: "2026-08-14T08:00:00Z",
        },
      ],
    });
    mocked.listCaseTasks.mockResolvedValue({
      items: [],
      page: 1,
      page_size: 25,
      total: 0,
    });

    render(<VolunteerWorkspacePage />);

    expect(await screen.findByText("Published v2 summary")).toBeVisible();
    expect(screen.getByText("Published v1 summary")).toBeVisible();
    expect(screen.getByText("Confirmed clue content")).toBeVisible();
    expect(screen.getByText(/Confirmed clue location/)).toBeVisible();
    expect(screen.getByText("Family-approved meeting point")).toBeVisible();
    expect(screen.getByText("Family-approved location text")).toBeVisible();
  });
});
