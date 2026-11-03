import { describe, expect, it } from "vitest";
import type { CaseTask, Clue } from "./api/cases";
import { getClueLocation, getTaskLocationText } from "./caseLocations";

describe("location display during migration", () => {
  it("uses the shared location instead of obsolete inline values", () => {
    const clue = {
      location_text: "旧地点",
      location: { location_text: "已核实地点", longitude: 120, latitude: 30 },
    } as Clue;
    expect(getClueLocation(clue).location_text).toBe("已核实地点");
    expect(getTaskLocationText({ area_text: "旧区域", location: clue.location } as CaseTask))
      .toBe("已核实地点");
  });

  it("does not restore stale location text when the shared location has no text", () => {
    expect(getTaskLocationText({ area_text: "旧区域", location: { location_text: null } } as CaseTask))
      .toBe("");
    expect(getClueLocation({ location_text: "旧服务器地点" } as Clue).location_text)
      .toBe("旧服务器地点");
  });
});
