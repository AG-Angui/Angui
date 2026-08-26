import { useEffect, useRef, useState } from 'react';
import AMapLoader from '@amap/amap-jsapi-loader';
import type { AMap } from '../types/amap';

interface UseAMapOptions {
  container: string | HTMLElement;
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
    let mounted = true;

    // 配置安全密钥（生产环境应使用代理）
    window._AMapSecurityConfig = {
      securityJsCode: import.meta.env.VITE_AMAP_SECURITY_KEY || import.meta.env.VITE_AMAP_JS_API_SECURITY_CODE || '',
      serviceHost: import.meta.env.VITE_AMAP_JS_API_SERVICE_HOST,
    };

    // 加载高德地图 JSAPI
    AMapLoader.load({
      key: import.meta.env.VITE_AMAP_KEY || import.meta.env.VITE_AMAP_JS_API_KEY || '',
      version: '2.0',
      plugins: ['AMap.Marker', 'AMap.InfoWindow'],
    })
      .then((AMap) => {
        if (!mounted) return;

        setAMapClass(AMap);

        // 创建地图实例
        const mapInstance = new AMap.Map(options.container, {
          viewMode: options.viewMode || '3D',
          zoom: options.zoom || 12,
          center: options.center || [116.397428, 39.90923],
          mapStyle: 'amap://styles/normal',
        });

        mapInstanceRef.current = mapInstance;
        setMap(mapInstance);
        setLoading(false);
      })
      .catch((err) => {
        if (!mounted) return;
        console.error('高德地图加载失败:', err);
        setError(err);
        setLoading(false);
      });

    // 清理函数：组件卸载时销毁地图实例
    return () => {
      mounted = false;
      if (mapInstanceRef.current) {
        mapInstanceRef.current.destroy();
        mapInstanceRef.current = null;
      }
    };
  }, [options.container]); // 仅在容器变化时重新初始化

  return { map, AMap: AMapClass, loading, error };
}
