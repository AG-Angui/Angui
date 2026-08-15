import { describe, expect, it } from "vitest";
import { parseOptionalCoordinatePair } from "./coordinateInput";

describe("parseOptionalCoordinatePair", () => {
  it("allows both coordinates to be omitted", () => {
    expect(parseOptionalCoordinatePair("  ", "")).toEqual({
      ok: true,
      longitude: null,
      latitude: null,
    });
  });

  it("requires a complete, finite coordinate pair", () => {
    expect(parseOptionalCoordinatePair("121.41", "")).toEqual({
      ok: false,
      message: "经度和纬度必须同时填写或同时留空。",
    });
    expect(parseOptionalCoordinatePair("-", "31.21")).toEqual({
      ok: false,
      message: "经度和纬度必须是有效数字。",
    });
  });

  it("enforces longitude and latitude ranges", () => {
    expect(parseOptionalCoordinatePair("180.01", "31.21")).toEqual({
      ok: false,
      message: "经度必须在 -180 到 180 之间。",
    });
    expect(parseOptionalCoordinatePair("121.41", "90.01")).toEqual({
      ok: false,
      message: "纬度必须在 -90 到 90 之间。",
    });
  });

  it("parses valid coordinate strings only when submitting", () => {
    expect(parseOptionalCoordinatePair(" 121.41 ", "31.21")).toEqual({
      ok: true,
      longitude: 121.41,
      latitude: 31.21,
    });
  });
});
