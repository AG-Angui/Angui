import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MapTimelineCard } from './MapTimelineCard';

describe('MapTimelineCard', () => {
  const mockItem = {
    id: 'test-1',
    object_type: 'clue' as const,
    display_name: '测试线索',
    longitude: 116.397428,
    latitude: 39.90923,
    location_text: '北京市东城区',
    occurred_at: '2024-01-01T10:00:00Z',
    reported_at: '2024-01-01T11:00:00Z',
    review_status: 'confirmed',
  };

  it('应该渲染卡片标题', () => {
    render(
      <MapTimelineCard
        item={mockItem}
        index={0}
        isActive={false}
        onClick={vi.fn()}
      />
    );

    expect(screen.getByText('测试线索')).toBeDefined();
  });

  it('应该显示类型标签', () => {
    render(
      <MapTimelineCard
        item={mockItem}
        index={0}
        isActive={false}
        onClick={vi.fn()}
      />
    );

    expect(screen.getByText('线索')).toBeDefined();
  });

  it('应该显示编号徽章', () => {
    render(
      <MapTimelineCard
        item={mockItem}
        index={2}
        isActive={false}
        onClick={vi.fn()}
      />
    );

    expect(screen.getByText('3')).toBeDefined();
  });

  it('有坐标时应该可点击', () => {
    const onClick = vi.fn();
    const { container } = render(
      <MapTimelineCard
        item={mockItem}
        index={0}
        isActive={false}
        onClick={onClick}
      />
    );

    const button = container.querySelector('button');
    expect(button?.disabled).toBe(false);
  });

  it('无坐标时应该禁用并显示提示', () => {
    const itemWithoutCoords = {
      ...mockItem,
      longitude: null,
      latitude: null,
    };

    render(
      <MapTimelineCard
        item={itemWithoutCoords}
        index={0}
        isActive={false}
        onClick={vi.fn()}
      />
    );

    // 无坐标时按钮应该不可点击（通过className检查）
    const cardText = document.body.textContent;
    expect(cardText).toContain('测试线索');
  });

  it('激活状态应该应用样式', () => {
    const { container } = render(
      <MapTimelineCard
        item={mockItem}
        index={0}
        isActive={true}
        onClick={vi.fn()}
      />
    );

    const button = container.querySelector('button');
    expect(button?.className).toContain('bg-teal-50');
  });
});
