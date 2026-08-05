import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { load } from "@amap/amap-jsapi-loader";
import { LocationConfirmationPicker } from "./LocationConfirmationPicker";

vi.mock("@amap/amap-jsapi-loader", () => ({
  load: vi.fn(),
}));

const amapLoad = vi.mocked(load);

function createAmapMock() {
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
          result: { info?: string; regeocode?: { formattedAddress?: string } },
        ) => void,
      ) =>
        callback("complete", {
          info: "OK",
          regeocode: {
            formattedAddress: `地点 ${longitude},${latitude}`,
          },
        }),
    ),
  };
  const placeSearch = {
    search: vi.fn(
      (
        _keyword: string,
        callback: (
          status: string,
          result: {
            poiList?: {
              pois?: Array<{
                id?: string;
                name?: string;
                address?: string;
                location?: { getLng(): number; getLat(): number };
              }>;
            };
          },
        ) => void,
      ) =>
        callback("complete", {
          poiList: {
            pois: [
              {
                id: "test-place",
                name: "测试地点",
                address: "测试地址",
                location: {
                  getLng: () => 116.4,
                  getLat: () => 39.92,
                },
              },
            ],
          },
        }),
    ),
  };
  const api = {
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
    PlaceSearch: class {
      search = placeSearch.search;
    },
    convertFrom: vi.fn(
      (
        [longitude, latitude]: [number, number],
        _source: "gps",
        callback: (
          status: string,
          result: {
            info?: string;
            locations?: Array<{ getLng(): number; getLat(): number }>;
          },
        ) => void,
      ) =>
        callback("complete", {
          info: "ok",
          locations: [
            {
              getLng: () => longitude + 0.01,
              getLat: () => latitude + 0.01,
            },
          ],
        }),
    ),
  };
  return {
    api,
    geocoder,
    placeSearch,
    markerListeners,
    getMapClick: () => mapClick,
  };
}

describe("LocationConfirmationPicker", () => {
  let amap: ReturnType<typeof createAmapMock>;

  beforeEach(() => {
    amap ??= createAmapMock();
    amapLoad.mockReset();
    amapLoad.mockResolvedValue(amap.api as never);
    vi.stubEnv("VITE_AMAP_JS_API_KEY", "test-key");
    vi.stubEnv("VITE_AMAP_JS_API_SERVICE_HOST", "/_AMapService");
    vi.stubEnv("VITE_AMAP_JS_API_SECURITY_CODE", "");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("allows retry after the loader rejects", async () => {
    amapLoad.mockRejectedValueOnce(new Error("地图服务加载失败"));
    const getCurrentPosition = vi.fn();
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });

    render(
      <LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));
    await act(async () => {
      getCurrentPosition.mock.calls[0][0]({
        coords: { longitude: 116.39, latitude: 39.91, accuracy: 80 },
      });
    });
    expect(await screen.findByText("地图服务加载失败")).toBeInTheDocument();
    expect(amapLoad).toHaveBeenCalledOnce();
    expect(amapLoad).toHaveBeenCalledWith({
      key: "test-key",
      version: "2.0",
      plugins: ["AMap.Geocoder", "AMap.PlaceSearch"],
    });
    expect(window._AMapSecurityConfig).toEqual({
      serviceHost: "/_AMapService",
    });

    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));
    await act(async () => {
      getCurrentPosition.mock.calls[1][0]({
        coords: { longitude: 116.39, latitude: 39.91, accuracy: 80 },
      });
    });
    await screen.findByLabelText("位置确认地图");
    expect(amapLoad).toHaveBeenCalledTimes(2);
  });

  it("does not fall back to unconverted coordinates when conversion is rejected", async () => {
    const originalConvertFrom = amap.api.convertFrom.getMockImplementation();
    amap.api.convertFrom.mockImplementation((_position, _source, callback) =>
      callback("complete", { info: "INVALID_USER_KEY" }),
    );
    const getCurrentPosition = vi.fn();
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });

    render(
      <LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));
    await act(async () => {
      getCurrentPosition.mock.calls[0][0]({
        coords: { longitude: 116.39, latitude: 39.91, accuracy: 80 },
      });
    });
    expect(
      await screen.findByText("无法完成坐标转换。你仍可手动填写地点。"),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("位置确认地图")).not.toBeInTheDocument();
    if (originalConvertFrom) {
      amap.api.convertFrom.mockImplementation(originalConvertFrom);
    }
  });

  it("requests location only after activation and confirms the converted, geocoded point", async () => {
    const getCurrentPosition = vi.fn();
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });
    const onConfirm = vi.fn();

    render(
      <LocationConfirmationPicker onConfirm={onConfirm} onClear={vi.fn()} />,
    );
    expect(getCurrentPosition).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));
    expect(getCurrentPosition).toHaveBeenCalledOnce();

    await act(async () => {
      getCurrentPosition.mock.calls[0][0]({
        coords: { longitude: 116.39, latitude: 39.91, accuracy: 20 },
      });
    });
    await screen.findByLabelText("位置确认地图");
    expect(screen.getByText(/地点 116\.4,39\.9/)).toBeInTheDocument();

    await act(async () => {
      amap.getMapClick()?.({
        lnglat: { getLng: () => 116.4, getLat: () => 39.92 },
      });
    });
    await act(async () => {
      amap.markerListeners.get("dragend")?.({
        lnglat: { getLng: () => 116.41, getLat: () => 39.93 },
      });
    });
    expect(screen.getByText("地点 116.41,39.93")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "确认此位置" }));
    expect(onConfirm).toHaveBeenCalledWith({
      address: "地点 116.41,39.93",
      longitude: 116.41,
      latitude: 39.93,
      accuracyMeters: 20,
      precision: "exact",
    });
  });

  it("searches a typed location before opening the map confirmation", async () => {
    render(
      <LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />,
    );

    fireEvent.change(screen.getByRole("textbox", { name: "搜索地点" }), {
      target: { value: "测试地点" },
    });
    fireEvent.click(screen.getByRole("button", { name: "搜索地点" }));

    expect(await screen.findByText("测试地点")).toBeInTheDocument();
    expect(amap.placeSearch.search).toHaveBeenLastCalledWith(
      "测试地点",
      expect.any(Function),
    );

    fireEvent.click(screen.getByRole("button", { name: /测试地点/ }));

    await screen.findByLabelText("位置确认地图");
    expect(screen.getByText("地点 116.4,39.92")).toBeInTheDocument();
  });

  it("clears stale results when the search keyword changes or is emptied", async () => {
    const originalSearch = amap.placeSearch.search.getMockImplementation();
    let pendingSearch:
      | ((
          status: string,
          result: {
            poiList?: {
              pois?: Array<{
                id?: string;
                name?: string;
                address?: string;
                location?: { getLng(): number; getLat(): number };
              }>;
            };
          },
        ) => void)
      | undefined;
    amap.placeSearch.search.mockImplementation((_keyword, callback) => {
      pendingSearch = callback;
    });

    render(
      <LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />,
    );

    fireEvent.change(screen.getByRole("textbox", { name: "搜索地点" }), {
      target: { value: "旧关键词" },
    });
    fireEvent.click(screen.getByRole("button", { name: "搜索地点" }));
    await waitFor(() => expect(pendingSearch).toBeDefined());

    fireEvent.change(screen.getByRole("textbox", { name: "搜索地点" }), {
      target: { value: "新关键词" },
    });
    await act(async () => {
      pendingSearch?.("complete", {
        poiList: {
          pois: [
            {
              id: "stale-place",
              name: "旧搜索结果",
              location: {
                getLng: () => 116.4,
                getLat: () => 39.92,
              },
            },
          ],
        },
      });
    });
    expect(screen.queryByText("旧搜索结果")).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("textbox", { name: "搜索地点" }), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "搜索地点" }));
    expect(
      screen.getByText("请输入地点、地标或场所后再搜索。"),
    ).toBeInTheDocument();

    if (originalSearch) {
      amap.placeSearch.search.mockImplementation(originalSearch);
    }
  });

  it("does not confirm when reverse geocoding fails and preserves manual entry", async () => {
    amap.geocoder.getAddress.mockImplementation((_position, callback) =>
      callback("complete", { info: "INVALID_USER_KEY" }),
    );
    const getCurrentPosition = vi.fn();
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });

    render(
      <LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));
    await act(async () => {
      getCurrentPosition.mock.calls[0][0]({
        coords: { longitude: 116.39, latitude: 39.91, accuracy: 80 },
      });
    });
    await screen.findByText("无法解析该点的文字地址。请改用手动填写地点。");
    expect(screen.getByRole("button", { name: "确认此位置" })).toBeDisabled();
  });

  it("keeps manual entry available after a denied permission request", () => {
    const getCurrentPosition = vi.fn(
      (_success: PositionCallback, failure: PositionErrorCallback) =>
        failure({ code: 1 } as GeolocationPositionError),
    );
    Object.defineProperty(navigator, "geolocation", {
      configurable: true,
      value: { getCurrentPosition },
    });

    render(
      <LocationConfirmationPicker onConfirm={vi.fn()} onClear={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "获取当前位置" }));

    expect(
      screen.getByText("定位权限未授予。你仍可手动填写地点。"),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("位置确认地图")).not.toBeInTheDocument();
  });
});
