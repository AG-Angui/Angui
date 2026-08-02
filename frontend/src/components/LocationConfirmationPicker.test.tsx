import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LocationConfirmationPicker } from "./LocationConfirmationPicker";

describe("LocationConfirmationPicker", () => {
  afterEach(() => {
    delete window.AMap;
  });

  it("requests location only after activation and confirms the final map point", async () => {
    let mapClick:
      | ((event: { lnglat: { getLng(): number; getLat(): number } }) => void)
      | undefined;
    const markerListeners = new Map<
      string,
      (event: { lnglat: { getLng(): number; getLat(): number } }) => void
    >();
    const geocoder = {
      getAddress: vi.fn(
        (
          [longitude, latitude]: [number, number],
          callback: (
            status: string,
            result: { regeocode?: { formattedAddress?: string } },
          ) => void,
        ) =>
          callback("complete", {
            regeocode: {
              formattedAddress: "地点 " + longitude + "," + latitude,
            },
          }),
      ),
    };
    window.AMap = {
      Map: class {
        destroy() {}
        on(
          name: string,
          listener: (event: {
            lnglat: { getLng(): number; getLat(): number };
          }) => void,
        ) {
          if (name === "click") mapClick = listener;
        }
        panTo() {}
      },
      Marker: class {
        on(
          name: string,
          listener: (event: {
            lnglat: { getLng(): number; getLat(): number };
          }) => void,
        ) {
          markerListeners.set(name, listener);
        }
        setMap() {}
        setPosition() {}
      },
      Geocoder: class {
        getAddress = geocoder.getAddress;
      },
      plugin(_plugins, callback) {
        callback();
      },
    };
    const getCurrentPosition = vi.fn();
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });
    const onConfirm = vi.fn();
    const onClear = vi.fn();

    render(<LocationConfirmationPicker onConfirm={onConfirm} onClear={onClear} />);

    expect(getCurrentPosition).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));
    expect(getCurrentPosition).toHaveBeenCalledOnce();

    await act(async () => {
      getCurrentPosition.mock.calls[0][0]({
        coords: { longitude: 116.39, latitude: 39.91 },
      });
    });
    await screen.findByLabelText("位置确认地图");

    await act(async () => {
      mapClick?.({
        lnglat: { getLng: () => 116.4, getLat: () => 39.92 },
      });
    });
    expect(markerListeners.get("dragend")).toBeDefined();
    expect(screen.getByText("地点 116.4,39.92")).toBeInTheDocument();

    await act(async () => {
      markerListeners.get("dragend")?.({
        lnglat: { getLng: () => 116.41, getLat: () => 39.93 },
      });
    });
    expect(screen.getByText("地点 116.41,39.93")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "确认此位置" }));
    expect(onConfirm).toHaveBeenCalledWith({
      address: "地点 116.41,39.93",
      longitude: 116.41,
      latitude: 39.93,
    });
    fireEvent.click(screen.getByRole("button", { name: "清除定位结果" }));
    expect(onClear).toHaveBeenCalledOnce();
    expect(screen.queryByLabelText("位置确认地图")).not.toBeInTheDocument();
  });

  it("keeps manual entry available after a denied permission request", () => {
    const getCurrentPosition = vi.fn(
      (
        _success: PositionCallback,
        failure: PositionErrorCallback,
      ) => failure({ code: 1 } as GeolocationPositionError),
    );
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });

    render(<LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));

    expect(
      screen.getByText("定位权限未授予。你仍可手动填写地点。"),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("位置确认地图")).not.toBeInTheDocument();
  });
});
