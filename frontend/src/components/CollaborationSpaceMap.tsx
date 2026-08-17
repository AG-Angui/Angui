import { useEffect, useRef, useState } from "react";
import { MapPinned } from "lucide-react";
import { load as loadAmapLibrary } from "@amap/amap-jsapi-loader";
import type { SpaceLocation, SpaceMember } from "../api/collaborationSpaces";

type AmapMap = { destroy(): void; setFitView?(overlays?: unknown[], immediately?: boolean, avoid?: number[]): void; setCenter(position: [number, number]): void };
type AmapMarker = { setMap(map: AmapMap | null): void };
type AmapApi = {
  Map: new (element: HTMLDivElement, options: { center: [number, number]; zoom: number }) => AmapMap;
  Marker: new (options: { position: [number, number]; title?: string; label?: { content: string; direction?: string } }) => AmapMarker;
};

declare global { interface Window { _AMapSecurityConfig?: { serviceHost?: string; securityJsCode?: string } } }

let amapPromise: Promise<AmapApi> | null = null;
function loadAmap(): Promise<AmapApi> {
  if (amapPromise) return amapPromise;
  const key = import.meta.env.VITE_AMAP_JS_API_KEY?.trim();
  if (!key) return Promise.reject(new Error("地图服务尚未配置"));
  const serviceHost = import.meta.env.VITE_AMAP_JS_API_SERVICE_HOST?.trim() || "/_AMapService";
  const securityJsCode = import.meta.env.DEV ? import.meta.env.VITE_AMAP_JS_API_SECURITY_CODE?.trim() : undefined;
  window._AMapSecurityConfig = { serviceHost, ...(securityJsCode ? { securityJsCode } : {}) };
  amapPromise = loadAmapLibrary({ key, version: "2.0" }).then((api) => api as AmapApi).catch((error) => { amapPromise = null; throw error; });
  return amapPromise;
}

export function CollaborationSpaceMap({ members, locations }: { members: SpaceMember[]; locations: SpaceLocation[] }) {
  const element = useRef<HTMLDivElement | null>(null);
  const map = useRef<AmapMap | null>(null);
  const markers = useRef<AmapMarker[]>([]);
  const [message, setMessage] = useState("");
  useEffect(() => {
    let cancelled = false;
    if (!element.current) return undefined;
    const latestByUser = new Map(locations.map((location) => [location.user_id, location]));
    const activeMembers = members.filter((member) => member.status === "active");
    void loadAmap().then((api) => {
      if (cancelled || !element.current) return;
      const first = locations[0];
      const nextMap = new api.Map(element.current, { center: first ? [first.longitude, first.latitude] : [116.397428, 39.90923], zoom: first ? 13 : 5 });
      map.current = nextMap;
      markers.current = activeMembers.flatMap((member) => {
        const location = latestByUser.get(member.user_id);
        if (!location) return [];
        const marker = new api.Marker({ position: [location.longitude, location.latitude], title: member.display_name, label: { content: member.display_name, direction: "top" } });
        marker.setMap(nextMap);
        return [marker];
      });
      if (markers.current.length > 1) nextMap.setFitView?.(markers.current, true, [40, 40, 40, 40]);
    }).catch((cause: unknown) => { if (!cancelled) setMessage(cause instanceof Error ? cause.message : "地图暂不可用"); });
    return () => { cancelled = true; markers.current.forEach((marker) => marker.setMap(null)); markers.current = []; map.current?.destroy(); map.current = null; };
  }, [locations, members]);

  return <div className="mt-3 overflow-hidden rounded-md border border-slate-200 bg-white" aria-label="协作成员位置地图">
    <div className="flex items-center gap-1 border-b border-slate-200 px-3 py-2 text-xs font-medium text-slate-700"><MapPinned size={14} />成员位置</div>
    <div ref={element} className="h-64 w-full bg-slate-100" />
    {message && <p className="m-0 px-3 py-2 text-xs text-amber-800" role="status">{message}</p>}
    {locations.length === 0 && !message && <p className="m-0 px-3 py-2 text-xs text-slate-500">暂无成员位置；成员开始共享后会显示在地图上。</p>}
  </div>;
}
