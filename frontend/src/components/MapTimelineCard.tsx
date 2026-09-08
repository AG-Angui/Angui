import { Button, Chip } from '@heroui/react';
import { MapPin } from 'lucide-react';

export interface MapTimelineCardProps {
  item: {
    id: string;
    object_type: string;
    display_name: string | null;
    longitude: number | null;
    latitude: number | null;
    location_text: string | null;
    occurred_at: string | null;
    reported_at: string | null;
    review_status: string;
  };
  index: number;
  isActive: boolean;
  onClick: () => void;
}

// 类型对应的颜色映射
const typeColors: Record<string, string> = {
  clue: '#d84343',        // 红色 - 线索
  task: '#2e8b57',        // 绿色 - 任务
  place: '#6a43cf',       // 紫色 - 地点
  last_seen: '#ef8f26',   // 橙色 - 最后出现
};

// 类型对应的中文标签
const typeLabels: Record<string, string> = {
  clue: '线索',
  task: '任务',
  place: '地点',
  last_seen: '最后出现',
};

/**
 * 地图时间线卡片组件
 * 显示单个地图项的详细信息，点击后地图飞到对应标记
 */
export function MapTimelineCard({
  item,
  index,
  isActive,
  onClick,
}: MapTimelineCardProps) {
  const hasCoordinates = item.longitude !== null && item.latitude !== null;
  const badgeColor = typeColors[item.object_type] || '#66717f';

  return (
    <Button
      className={`w-full p-3 text-left transition-all ${
        isActive
          ? 'bg-teal-50 border-teal-300 shadow-md'
          : 'bg-white/70 border-transparent hover:border-teal-200 hover:shadow-sm'
      }`}
      variant="ghost"
      onClick={hasCoordinates ? onClick : undefined}
      isDisabled={!hasCoordinates}
      style={{ borderWidth: '1px', borderRadius: '16px' }}
    >
      <div className="grid grid-cols-[40px_1fr] gap-3 w-full">
        {/* 编号徽章 */}
        <div
          className="w-10 h-10 rounded-xl flex items-center justify-center text-white font-bold text-base"
          style={{ backgroundColor: badgeColor }}
        >
          {index + 1}
        </div>

        {/* 卡片内容 */}
        <div className="flex flex-col gap-1 min-w-0">
          <h3 className="font-bold text-sm leading-tight truncate">
            {item.display_name || '未命名项目'}
          </h3>

          <div className="flex items-center gap-2 flex-wrap">
            <Chip size="sm" variant="soft" color="default">
              {typeLabels[item.object_type] || item.object_type}
            </Chip>
            {item.occurred_at && (
              <span className="text-xs text-gray-500">
                {new Date(item.occurred_at).toLocaleString('zh-CN', {
                  month: 'short',
                  day: 'numeric',
                  hour: '2-digit',
                  minute: '2-digit',
                })}
              </span>
            )}
          </div>

          {/* 位置信息 */}
          <p className="text-xs text-gray-600 line-clamp-2">
            {hasCoordinates ? (
              <>
                <MapPin className="inline w-3 h-3 mr-1" />
                {item.location_text || `${item.latitude}, ${item.longitude}`}
              </>
            ) : (
              <>
                <span className="text-gray-400">📍 仅文字位置</span>
                <span className="ml-2">{item.location_text || '无位置信息'}</span>
              </>
            )}
          </p>
        </div>
      </div>
    </Button>
  );
}
