import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CaseLocation } from "../api/cases";
import { CaseLocationSelect } from "./CaseLocationSelect";

const location: CaseLocation = {
  id: "shared-location",
  case_id: "fictional-case",
  location_text: "模拟公园北门",
  location_precision: "exact",
  location_kind: "point",
  longitude: 120,
  latitude: 30,
  location_radius_meters: null,
};

describe("existing case location selection", () => {
  it("offers one choice for a shared location and switches back to a new location", () => {
    const onChange = vi.fn();
    render(<CaseLocationSelect locations={[location, null, location]} value="" onChange={onChange} />);
    expect(screen.getAllByRole("option")).toHaveLength(2);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: location.id } });
    expect(onChange).toHaveBeenLastCalledWith(location);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "" } });
    expect(onChange).toHaveBeenLastCalledWith(null);
  });

  it("does not offer a location when none is present in authorized records", () => {
    render(<CaseLocationSelect locations={[null, undefined]} value="" onChange={vi.fn()} />);
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });
});
