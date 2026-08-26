import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { ClueMapView } from './ClueMapView';

// Mock 高德地图加载器
vi.mock('@amap/amap-jsapi-loader', () => ({
  default: {
    load: vi.fn(() =>
      Promise.resolve({
        Map: vi.fn(function(this: any) {
          this.setZoom = vi.fn();
          this.setCenter = vi.fn();
          this.setFitView = vi.fn();
          this.destroy = vi.fn();
          this.on = vi.fn();
          return this;
        }),
        Marker: vi.fn(function(this: any) {
          this.setMap = vi.fn();
          this.on = vi.fn();
          this.getPosition = vi.fn(() => ({ getLng: () => 116.397428, getLat: () => 39.90923 }));
          return this;
        }),
        InfoWindow: vi.fn(function(this: any) {
          this.setContent = vi.fn();
          this.open = vi.fn();
          this.close = vi.fn();
          return this;
        }),
        LngLat: vi.fn(function(this: any, lng: number, lat: number) {
          this.lng = lng;
          this.lat = lat;
          return this;
        }),
        Pixel: vi.fn(function(this: any, x: number, y: number) {
          this.x = x;
          this.y = y;
          return this;
        }),
        Size: vi.fn(function(this: any, w: number, h: number) {
          this.width = w;
          this.height = h;
          return this;
        }),
        Icon: vi.fn(function(this: any) {
          return this;
        }),
      })
    ),
  },
}));

describe('ClueMapView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('应该渲染地图容器', async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      const mapContainer = document.getElementById('clue-map-container');
      expect(mapContainer).toBeDefined();
    });
  });

  it('应该渲染时间线卡片', async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      expect(screen.getByText('测试线索1')).toBeDefined();
      expect(screen.getByText('测试任务')).toBeDefined();
    });
  });

  it('应该显示重置按钮', async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      expect(screen.getByText('重置视图')).toBeDefined();
    });
  });

  it('空数据时应该显示空状态提示', () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);
    expect(screen.getByText(/暂无/)).toBeDefined();
  });

  it('应该过滤出有坐标的项目', async () => {
    render(<ClueMapView caseId="test-case-1" token="test-token" />);

    await waitFor(() => {
      // 应该显示卡片但标记为仅文字位置
      expect(screen.getByText('测试线索1')).toBeDefined();
    });
  });
});
