/**
 * 高德地图 JSAPI v2.0 TypeScript 类型定义
 * 包含本项目使用到的核心类型
 */

declare global {
  interface Window {
    _AMapSecurityConfig?: {
      securityJsCode: string;
      serviceHost?: string;
    };
    AMap?: typeof AMap;
  }
}

export namespace AMap {
  // 基础类
  class LngLat {
    constructor(lng: number, lat: number);
    lng: number;
    lat: number;
  }

  class Bounds {
    constructor(southWest: LngLat, northEast: LngLat);
    extend(lngLat: LngLat): void;
    contains(lngLat: LngLat): boolean;
  }

  // 地图类
  interface MapOptions {
    viewMode?: '2D' | '3D';
    zoom?: number;
    center?: [number, number] | LngLat;
    pitch?: number;
    rotation?: number;
    mapStyle?: string;
  }

  class Map {
    constructor(container: string | HTMLElement, options?: MapOptions);

    // 视图控制
    setZoom(zoom: number): void;
    getZoom(): number;
    setCenter(center: [number, number] | LngLat): void;
    getCenter(): LngLat;
    setBounds(bounds: Bounds): void;
    setFitView(overlays?: Overlay[], immediately?: boolean): void;

    // 覆盖物管理
    add(overlay: Overlay | Overlay[]): void;
    remove(overlay: Overlay | Overlay[]): void;
    clearMap(): void;

    // 事件
    on(event: string, handler: (event: any) => void): void;
    off(event: string, handler: (event: any) => void): void;

    // 销毁
    destroy(): void;
  }

  // 标记类
  interface MarkerOptions {
    position?: [number, number] | LngLat;
    icon?: string | Icon;
    content?: string | HTMLElement;
    title?: string;
    offset?: Pixel;
    anchor?: string;
    draggable?: boolean;
    extData?: any;
  }

  class Marker extends Overlay {
    constructor(options?: MarkerOptions);
    setPosition(position: [number, number] | LngLat): void;
    getPosition(): LngLat;
    setIcon(icon: string | Icon): void;
    setContent(content: string | HTMLElement): void;
    setExtData(data: any): void;
    getExtData(): any;
  }

  // 图标类
  interface IconOptions {
    size?: Size;
    image?: string;
    imageSize?: Size;
  }

  class Icon {
    constructor(options?: IconOptions);
  }

  // 信息窗体
  interface InfoWindowOptions {
    content?: string | HTMLElement;
    offset?: Pixel;
    position?: [number, number] | LngLat;
  }

  class InfoWindow {
    constructor(options?: InfoWindowOptions);
    open(map: Map, position?: [number, number] | LngLat): void;
    close(): void;
    setContent(content: string | HTMLElement): void;
  }

  // 辅助类
  class Pixel {
    constructor(x: number, y: number);
    x: number;
    y: number;
  }

  class Size {
    constructor(width: number, height: number);
    width: number;
    height: number;
  }

  // 覆盖物基类
  class Overlay {
    setMap(map: Map | null): void;
    on(event: string, handler: (event: any) => void): void;
    off(event: string, handler: (event: any) => void): void;
  }
}

export {};
