import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import AMapLoader from "@amap/amap-jsapi-loader";
import { ClueMapView } from "./ClueMapView";

// Mock API
vi.mock("../api/cases", () => ({
  getCaseMapView: vi.fn(() =>
    Promise.resolve({
      items: [
        {
          id: "clue-1",
          object_type: "clue",
          display_name: "测试线索1",
          longitude: 116.397428,
          latitude: 39.90923,
          location_text: "北京市东城区",
          location_precision: "exact",
          source: "family",
          occurred_at: "2024-01-01T10:00:00Z",
          reported_at: "2024-01-01T11:00:00Z",
          review_status: "confirmed",
          related_task_id: null,
          updated_at: "2024-01-01T12:00:00Z",
        },
        {
          id: "task-1",
          object_type: "task",
          display_name: "测试任务",
          longitude: 116.407428,
          latitude: 39.91923,
          location_text: "北京市朝阳区",
          location_precision: "exact",
          source: "commander",
          occurred_at: null,
          reported_at: "2024-01-02T10:00:00Z",
          review_status: "pending_review",
          related_task_id: null,
          updated_at: "2024-01-02T10:00:00Z",
        },
      ],
    }),
  ),
}));

// Mock 高德地图加载器
vi.mock("@amap/amap-jsapi-loader", () => ({
  default: {
    load: vi.fn(() =>
      Promise.resolve({
        Map: vi.fn(function (this: any) {
          this.setZoom = vi.fn();
          this.setCenter = vi.fn();
          this.setFitView = vi.fn();
          this.destroy = vi.fn();
          this.on = vi.fn();
          return this;
        }),
        Marker: vi.fn(function (this: any) {
          this.setMap = vi.fn();
          this.on = vi.fn();
          this.getPosition = vi.fn(() => ({
            getLng: () => 116.397428,
            getLat: () => 39.90923,
          }));
          return this;
        }),
        InfoWindow: vi.fn(function (this: any) {
          this.setContent = vi.fn();
          this.open = vi.fn();
          this.close = vi.fn();
          return this;
        }),
        LngLat: vi.fn(function (this: any, lng: number, lat: number) {
          this.lng = lng;
          this.lat = lat;
          return this;
        }),
        Pixel: vi.fn(function (this: any, x: number, y: number) {
          this.x = x;
          this.y = y;
          return this;
        }),
        Size: vi.fn(function (this: any, w: number, h: number) {
          this.width = w;
          this.height = h;
          return this;
        }),
        Icon: vi.fn(function (this: any) {
          return this;
        }),
      }),
    ),
  },
}));

describe("ClueMapView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("应该渲染地图容器", async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      const mapContainer = screen.getByTestId("clue-map-container");
      expect(mapContainer).toBeDefined();
    });
  });

  it("应该渲染时间线卡片", async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      expect(screen.getByText("测试线索1")).toBeDefined();
      expect(screen.getByText("测试任务")).toBeDefined();
    });
  });

  it("应该显示重置按钮", async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      expect(screen.getByText("重置视图")).toBeDefined();
    });
  });

  it("空数据时应该显示空状态提示", async () => {
    const { getCaseMapView } = await import("../api/cases");
    (getCaseMapView as any).mockResolvedValueOnce({ items: [] });

    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      expect(screen.getByText(/暂无/)).toBeDefined();
    });
  });

  it("应该过滤出有坐标的项目", async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      // 应该显示卡片但标记为仅文字位置
      expect(screen.getByText("测试线索1")).toBeDefined();
    });
  });

  it("shares a marker between linked records while keeping both timeline cards", async () => {
    const { getCaseMapView } = await import("../api/cases");
    const item = {
      location_id: "shared-location",
      object_type: "clue" as const,
      display_name: "模拟目击记录",
      longitude: 120,
      latitude: 30,
      location_text: "模拟公园北门",
      location_precision: "exact" as const,
      source: "family",
      occurred_at: null,
      reported_at: null,
      review_status: "confirmed",
      related_task_id: null,
      updated_at: "2026-11-03T00:00:00Z",
    };
    vi.mocked(getCaseMapView).mockResolvedValueOnce({ items: [
      { ...item, id: "clue-a" },
      { ...item, id: "clue-b", display_name: "模拟后续记录" },
    ], areas: [] });
    render(<ClueMapView caseId="fictional-case" token="test-token" />);
    await screen.findByText("模拟后续记录");
    const amap = await vi.mocked(AMapLoader.load).mock.results.at(-1)!.value;
    await waitFor(() => expect(amap.Marker).toHaveBeenCalledTimes(1));
    expect(screen.getByText("模拟目击记录")).toBeInTheDocument();
    fireEvent.click(screen.getByText("模拟后续记录"));
    const map = vi.mocked(amap.Map).mock.results[0]!.value;
    expect(map.setCenter).toHaveBeenCalled();
  });
});
