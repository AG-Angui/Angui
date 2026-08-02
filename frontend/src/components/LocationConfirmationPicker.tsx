import { Button } from "@heroui/react";
import { LocateFixed, MapPin, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

export type ConfirmedLocation = {
  address: string;
  latitude: number;
  longitude: number;
};

type CandidateLocation = ConfirmedLocation;

type AmapEvent = {
  lnglat?: {
    getLat(): number;
    getLng(): number;
  };
};

type AmapMarker = {
  on(name: string, listener: (event: AmapEvent) => void): void;
  setMap(map: AmapMap): void;
  setPosition(position: [number, number]): void;
};

type AmapMap = {
  destroy(): void;
  on(name: string, listener: (event: AmapEvent) => void): void;
  panTo(position: [number, number]): void;
};

type AmapApi = {
  Map: new (
    element: HTMLDivElement,
    options: { center: [number, number]; zoom: number },
  ) => AmapMap;
  Marker: new (options: {
    position: [number, number];
    draggable: boolean;
  }) => AmapMarker;
  Geocoder: new (options?: {
    radius?: number;
    extensions?: "base";
  }) => {
    getAddress(
      position: [number, number],
      callback: (
        status: string,
        result: { regeocode?: { formattedAddress?: string } },
      ) => void,
    ): void;
  };
  convertFrom?(
    position: [number, number],
    source: "gps",
    callback: (
      status: string,
      result: {
        locations?: Array<{
          getLat(): number;
          getLng(): number;
        }>;
      },
    ) => void,
  ): void;
  plugin(plugins: string[], callback: () => void): void;
};

declare global {
  interface Window {
    AMap?: AmapApi;
  }
}

const amapScriptId = "angui-amap-js-api";
let amapPromise: Promise<AmapApi> | null = null;

function coordinateAddress(longitude: number, latitude: number) {
  return "坐标 " + longitude.toFixed(6) + ", " + latitude.toFixed(6);
}

function browserLocationError(error: GeolocationPositionError) {
  switch (error.code) {
    case 1:
      return "定位权限未授予。你仍可手动填写地点。";
    case 2:
      return "暂时无法取得当前位置。你仍可手动填写地点。";
    case 3:
      return "定位请求超时。你仍可手动填写地点。";
    default:
      return "无法取得当前位置。你仍可手动填写地点。";
  }
}

function loadAmap() {
  if (window.AMap) return Promise.resolve(window.AMap);
  if (amapPromise) return amapPromise;

  const key = import.meta.env.VITE_AMAP_JS_API_KEY?.trim();
  if (!key) {
    return Promise.reject(
      new Error("地图服务尚未配置。你仍可手动填写地点。"),
    );
  }

  amapPromise = new Promise<AmapApi>((resolve, reject) => {
    const existing = document.getElementById(amapScriptId);
    if (existing) {
      existing.addEventListener("load", () => {
        if (window.AMap) resolve(window.AMap);
        else reject(new Error("地图服务加载失败。你仍可手动填写地点。"));
      });
      existing.addEventListener("error", () =>
        reject(new Error("地图服务加载失败。你仍可手动填写地点。")),
      );
      return;
    }

    const script = document.createElement("script");
    script.id = amapScriptId;
    script.async = true;
    script.src =
      "https://webapi.amap.com/maps?v=2.0&key=" + encodeURIComponent(key);
    script.onload = () => {
      if (window.AMap) resolve(window.AMap);
      else reject(new Error("地图服务加载失败。你仍可手动填写地点。"));
    };
    script.onerror = () =>
      reject(new Error("地图服务加载失败。你仍可手动填写地点。"));
    document.head.append(script);
  });

  return amapPromise;
}

export function LocationConfirmationPicker({
  onConfirm,
  onClear,
}: {
  onConfirm: (location: ConfirmedLocation) => void;
  onClear: () => void;
}) {
  const mapElement = useRef<HTMLDivElement | null>(null);
  const map = useRef<AmapMap | null>(null);
  const marker = useRef<AmapMarker | null>(null);
  const amap = useRef<AmapApi | null>(null);
  const candidateRef = useRef<CandidateLocation | null>(null);
  const geocodeRequest = useRef(0);
  const [candidate, setCandidate] = useState<CandidateLocation | null>(null);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [isLocating, setIsLocating] = useState(false);
  const [message, setMessage] = useState("");

  const resolveAddress = useCallback((longitude: number, latitude: number) => {
    const api = amap.current;
    const request = geocodeRequest.current + 1;
    geocodeRequest.current = request;
    if (!api) return;

    api.plugin(["AMap.Geocoder"], () => {
      const geocoder = new api.Geocoder({ radius: 1_000, extensions: "base" });
      geocoder.getAddress([longitude, latitude], (status, result) => {
        if (request !== geocodeRequest.current) return;
        const address =
          status === "complete" ? result.regeocode?.formattedAddress?.trim() : "";
        if (!address) {
          setMessage("无法解析该点的文字地址。你可确认坐标，或关闭后手动填写地点。");
          return;
        }
        setCandidate((current) => {
          if (
            !current ||
            current.longitude !== longitude ||
            current.latitude !== latitude
          ) {
            return current;
          }
          const next = { ...current, address };
          candidateRef.current = next;
          return next;
        });
      });
    });
  }, []);

  const chooseCoordinates = useCallback(
    (longitude: number, latitude: number) => {
      const next = {
        longitude,
        latitude,
        address: coordinateAddress(longitude, latitude),
      };
      candidateRef.current = next;
      setCandidate(next);
      marker.current?.setPosition([longitude, latitude]);
      map.current?.panTo([longitude, latitude]);
      resolveAddress(longitude, latitude);
    },
    [resolveAddress],
  );

  useEffect(() => {
    if (
      !isPickerOpen ||
      !candidateRef.current ||
      map.current ||
      !mapElement.current
    ) {
      return;
    }
    let cancelled = false;

    void loadAmap()
      .then((api) => {
        if (cancelled || !mapElement.current || !candidateRef.current) return;
        amap.current = api;
        const initial = candidateRef.current;
        const nextMap = new api.Map(mapElement.current, {
          center: [initial.longitude, initial.latitude],
          zoom: 16,
        });
        const nextMarker = new api.Marker({
          position: [initial.longitude, initial.latitude],
          draggable: true,
        });
        nextMarker.setMap(nextMap);
        nextMap.on("click", (event) => {
          const longitude = event.lnglat?.getLng();
          const latitude = event.lnglat?.getLat();
          if (longitude === undefined || latitude === undefined) return;
          chooseCoordinates(longitude, latitude);
        });
        nextMarker.on("dragend", (event) => {
          const longitude = event.lnglat?.getLng();
          const latitude = event.lnglat?.getLat();
          if (longitude === undefined || latitude === undefined) return;
          chooseCoordinates(longitude, latitude);
        });
        map.current = nextMap;
        marker.current = nextMarker;
        resolveAddress(initial.longitude, initial.latitude);
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setMessage(
            cause instanceof Error
              ? cause.message
              : "地图服务暂不可用。你仍可手动填写地点。",
          );
        }
      });

    return () => {
      cancelled = true;
      map.current?.destroy();
      map.current = null;
      marker.current = null;
      amap.current = null;
    };
  }, [chooseCoordinates, isPickerOpen, resolveAddress]);

  function requestLocation() {
    setMessage("");
    if (!navigator.geolocation) {
      setMessage("此设备不支持系统定位。你仍可手动填写地点。");
      return;
    }

    setIsLocating(true);
    navigator.geolocation.getCurrentPosition(
      ({ coords }) => {
        setIsLocating(false);
        void loadAmap()
          .then((api) => {
            amap.current = api;
            const openPicker = (longitude: number, latitude: number) => {
              chooseCoordinates(longitude, latitude);
              setIsPickerOpen(true);
            };
            if (!api.convertFrom) {
              openPicker(coords.longitude, coords.latitude);
              return;
            }
            api.convertFrom(
              [coords.longitude, coords.latitude],
              "gps",
              (status, result) => {
                const point =
                  status === "complete" ? result.locations?.[0] : undefined;
                openPicker(
                  point?.getLng() ?? coords.longitude,
                  point?.getLat() ?? coords.latitude,
                );
              },
            );
          })
          .catch((cause: unknown) =>
            setMessage(
              cause instanceof Error
                ? cause.message
                : "地图服务暂不可用。你仍可手动填写地点。",
            ),
          );
      },
      (error) => {
        setIsLocating(false);
        setMessage(browserLocationError(error));
      },
      { enableHighAccuracy: true, timeout: 12_000, maximumAge: 30_000 },
    );
  }

  function clear() {
    geocodeRequest.current += 1;
    candidateRef.current = null;
    setCandidate(null);
    setIsPickerOpen(false);
    setMessage("");
    onClear();
  }

  return (
    <div className="grid gap-2">
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          size="sm"
          variant="secondary"
          isDisabled={isLocating}
          onPress={requestLocation}
        >
          <LocateFixed size={16} />
          {isLocating ? "正在获取当前位置" : "获取当前位置"}
        </Button>
        {candidate && (
          <Button type="button" size="sm" variant="ghost" onPress={clear}>
            <X size={16} />
            清除定位结果
          </Button>
        )}
      </div>
      {message && (
        <p className="m-0 text-xs text-amber-800" role="status">
          {message}
        </p>
      )}
      {isPickerOpen && candidate && (
        <div className="overflow-hidden rounded-md border border-slate-200 bg-white">
          <div
            ref={mapElement}
            aria-label="位置确认地图"
            className="h-64 w-full bg-slate-100"
          />
          <div className="grid gap-2 border-t border-slate-200 px-3 py-3 text-sm">
            <p className="m-0 flex items-start gap-2 text-slate-700">
              <MapPin className="mt-0.5 shrink-0 text-brand-700" size={16} />
              <span>{candidate.address}</span>
            </p>
            <p className="m-0 font-mono text-xs text-slate-500">
              {candidate.longitude.toFixed(6)}, {candidate.latitude.toFixed(6)}
            </p>
            <p className="m-0 text-xs text-slate-500">
              可拖动标记或点击地图重新选点，确认后才会回填表单。
            </p>
            <div className="flex justify-end">
              <Button
                type="button"
                size="sm"
                variant="primary"
                onPress={() => {
                  if (candidateRef.current) onConfirm(candidateRef.current);
                }}
              >
                确认此位置
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
