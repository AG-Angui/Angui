import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '@heroui/react';
import { RotateCcw } from 'lucide-react';
import { useAMap } from '../hooks/useAMap';
import { MapTimelineCard } from './MapTimelineCard';
import type { AMap as AMapType } from '../types/amap';
import { LoadingState, ErrorState, EmptyState } from './ContentState';

export interface CaseMapItem {
  id: string;
  object_type: 'clue' | 'task' | 'place' | 'last_seen';
  display_name: string | null;
  longitude: number | null;
  latitude: number | null;
  location_text: string | null;
  location_precision: string;
  source: string;
  occurred_at: string | null;
  reported_at: string | null;
  review_status: string;
  related_task_id: string | null;
  updated_at: string;
}

export interface ClueMapViewProps {
  items: CaseMapItem[];
  className?: string;
}

// 类型对应的标记颜色
const markerColors: Record<string, string> = {
  clue: '#d84343',
  task: '#2e8b57',
  place: '#6a43cf',
  last_seen: '#ef8f26',
};

/**
 * 线索地图视图组件
 * 使用高德地图展示线索、任务和地点，支持卡片与地图标记的交互同步
 */
export function ClueMapView({ items, className = '' }: ClueMapViewProps) {
  const mapContainerId = 'clue-map-container';
  const { map, AMap, loading, error } = useAMap({
    container: mapContainerId,
    zoom: 12,
    viewMode: '3D',
  });

  const [activeItemId, setActiveItemId] = useState<string | null>(null);
  const markersRef = useRef<Map<string, AMapType.Marker>>(new Map());
  const infoWindowRef = useRef<AMapType.InfoWindow | null>(null);

  // 过滤出有坐标的项目
  const itemsWithCoords = items.filter(
    (item) => item.longitude !== null && item.latitude !== null
  );

  // 创建自定义标记图标
  const createMarkerIcon = useCallback(
    (type: string, index: number) => {
      if (!AMap) return undefined;

      const color = markerColors[type] || '#66717f';
      const canvas = document.createElement('canvas');
      canvas.width = 40;
      canvas.height = 50;
      const ctx = canvas.getContext('2d');
      if (!ctx) return undefined;

      // 绘制水滴形状标记
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(20, 16, 14, 0, Math.PI * 2);
      ctx.fill();
      ctx.beginPath();
      ctx.moveTo(20, 30);
      ctx.lineTo(14, 42);
      ctx.quadraticCurveTo(20, 50, 26, 42);
      ctx.closePath();
      ctx.fill();

      // 绘制编号文字
      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 14px sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(String(index + 1), 20, 16);

      return new AMap.Icon({
        image: canvas.toDataURL(),
        size: new AMap.Size(40, 50),
        imageSize: new AMap.Size(40, 50),
      });
    },
    [AMap]
  );

  // 初始化地图标记
  useEffect(() => {
    if (!map || !AMap || itemsWithCoords.length === 0) return;

    // 清除旧标记
    markersRef.current.forEach((marker) => marker.setMap(null));
    markersRef.current.clear();

    // 创建信息窗体
    if (!infoWindowRef.current) {
      infoWindowRef.current = new AMap.InfoWindow({
        offset: new AMap.Pixel(0, -30),
      });
    }

    // 添加新标记
    itemsWithCoords.forEach((item, index) => {
      if (item.longitude === null || item.latitude === null) return;

      const position = new AMap.LngLat(item.longitude, item.latitude);
      const icon = createMarkerIcon(item.object_type, index);

      const marker = new AMap.Marker({
        position,
        icon,
        extData: { itemId: item.id },
      });

      // 点击标记显示信息窗体
      marker.on('click', () => {
        const content = `
          <div style="padding: 12px; min-width: 200px;">
            <h3 style="margin: 0 0 8px; font-size: 15px; font-weight: bold;">
              ${item.display_name || '未命名'}
            </h3>
            <p style="margin: 4px 0; font-size: 13px; color: #666;">
              ${item.location_text || ''}
            </p>
            ${
              item.occurred_at
                ? `<p style="margin: 4px 0; font-size: 12px; color: #999;">
                时间: ${new Date(item.occurred_at).toLocaleString('zh-CN')}
              </p>`
                : ''
            }
          </div>
        `;

        infoWindowRef.current?.setContent(content);
        infoWindowRef.current?.open(map, position);
        setActiveItemId(item.id);
      });

      marker.setMap(map);
      markersRef.current.set(item.id, marker);
    });

    // 自适应显示所有标记
    if (itemsWithCoords.length > 0) {
      map.setFitView(Array.from(markersRef.current.values()));
    }
  }, [map, AMap, itemsWithCoords, createMarkerIcon]);

  // 卡片点击处理：飞到对应标记
  const handleCardClick = useCallback(
    (item: CaseMapItem) => {
      if (!map || item.longitude === null || item.latitude === null) return;

      const marker = markersRef.current.get(item.id);
      if (marker) {
        const position = marker.getPosition();
        map.setZoom(16);
        map.setCenter(position);
        setActiveItemId(item.id);

        // 自动打开信息窗体
        setTimeout(() => {
          const content = `
            <div style="padding: 12px; min-width: 200px;">
              <h3 style="margin: 0 0 8px; font-size: 15px; font-weight: bold;">
                ${item.display_name || '未命名'}
              </h3>
              <p style="margin: 4px 0; font-size: 13px; color: #666;">
                ${item.location_text || ''}
              </p>
              ${
                item.occurred_at
                  ? `<p style="margin: 4px 0; font-size: 12px; color: #999;">
                  时间: ${new Date(item.occurred_at).toLocaleString('zh-CN')}
                </p>`
                  : ''
              }
            </div>
          `;

          infoWindowRef.current?.setContent(content);
          infoWindowRef.current?.open(map, position);
        }, 300);
      }
    },
    [map]
  );

  // 重置视图：显示所有标记
  const handleResetView = useCallback(() => {
    if (!map || itemsWithCoords.length === 0) return;
    map.setFitView(Array.from(markersRef.current.values()));
    setActiveItemId(null);
    infoWindowRef.current?.close();
  }, [map, itemsWithCoords.length]);

  if (loading) {
    return <LoadingState message="正在加载地图..." />;
  }

  if (error) {
    return (
      <ErrorState
        message="地图加载失败"
        details={error.message}
        onRetry={() => window.location.reload()}
      />
    );
  }

  if (items.length === 0) {
    return <EmptyState message="暂无地图数据" />;
  }

  return (
    <div className={`grid grid-cols-1 lg:grid-cols-[340px_1fr] gap-4 h-full ${className}`}>
      {/* 侧边栏：时间线卡片列表 */}
      <aside className="flex flex-col gap-4 overflow-hidden">
        <div className="flex-1 overflow-y-auto space-y-2 pr-2">
          {items.map((item, index) => (
            <MapTimelineCard
              key={item.id}
              item={item}
              index={index}
              isActive={activeItemId === item.id}
              onClick={() => handleCardClick(item)}
            />
          ))}
        </div>

        <Button
          color="primary"
          variant="flat"
          startContent={<RotateCcw className="w-4 h-4" />}
          onClick={handleResetView}
          className="flex-shrink-0"
        >
          回到总览
        </Button>
      </aside>

      {/* 地图容器 */}
      <div
        id={mapContainerId}
        className="w-full h-full min-h-[500px] rounded-2xl overflow-hidden shadow-lg"
      />
    </div>
  );
}
