import { useEffect, useRef, useState } from 'react';
import AMapLoader from '@amap/amap-jsapi-loader';
import type { AMap } from '../types/amap';

interface UseAMapOptions {
  container: HTMLElement | null;
  center?: [number, number];
  zoom?: number;
  viewMode?: '2D' | '3D';
}

interface UseAMapReturn {
  map: AMap.Map | null;
  AMap: typeof AMap | null;
  loading: boolean;
  error: Error | null;
}

/**
 * 高德地图自定义 Hook
 * 负责地图实例的初始化、生命周期管理和清理
 */
export function useAMap(options: UseAMapOptions): UseAMapReturn {
  const [map, setMap] = useState<AMap.Map | null>(null);
  const [AMapClass, setAMapClass] = useState<typeof AMap | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const mapInstanceRef = useRef<AMap.Map | null>(null);

  useEffect(() => {
    const container = options.container;
    if (!container) return;

    let active = true;
    let animationFrameId: number | null = null;
    let mapInstance: AMap.Map | null = null;
    setLoading(true);
    setError(null);

    window._AMapSecurityConfig = {
      securityJsCode: import.meta.env.VITE_AMAP_JS_API_SECURITY_CODE || '',
      serviceHost: import.meta.env.VITE_AMAP_JS_API_SERVICE_HOST,
    };

    AMapLoader.load({
      key: import.meta.env.VITE_AMAP_JS_API_KEY || '',
      version: '2.0',
      plugins: ['AMap.Marker', 'AMap.InfoWindow'],
    })
      .then((AMap) => {
        // Give React one paint to finish a concurrent route, tab, or case switch.
        // The container may still be connected when the loader resolves but be
        // replaced before the browser paints the next frame.
        animationFrameId = window.requestAnimationFrame(() => {
          if (!active || !container.isConnected || !document.contains(container)) {
            return;
          }

          setAMapClass(AMap);
          mapInstance = new AMap.Map(container, {
            viewMode: options.viewMode || '3D',
            zoom: options.zoom || 12,
            center: options.center || [116.397428, 39.90923],
            mapStyle: 'amap://styles/normal',
          });
          mapInstanceRef.current = mapInstance;
          setMap(mapInstance);
          setLoading(false);
        });
      })
      .catch((cause: Error) => {
        if (!active) return;
        console.error('高德地图加载失败:', cause);
        setError(cause);
        setLoading(false);
      });

    return () => {
      active = false;
      if (animationFrameId !== null) {
        window.cancelAnimationFrame(animationFrameId);
      }
      mapInstance?.destroy();
      if (mapInstanceRef.current === mapInstance) {
        mapInstanceRef.current = null;
        setMap(null);
      }
    };
  }, [options.container]);
  return { map, AMap: AMapClass, loading, error };
}
