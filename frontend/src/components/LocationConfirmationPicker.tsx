import { load as loadAmapLibrary } from "@amap/amap-jsapi-loader";
import { Button } from "@heroui/react";
import { LocateFixed, MapPin, Search, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

export type LocationPrecision = "exact" | "approximate";

export type ConfirmedLocation = {
  address: string;
  latitude: number;
  longitude: number;
  accuracyMeters?: number;
  precision: LocationPrecision;
};

type AmapEvent = {
  lnglat?: {
    getLat(): number;
    getLng(): number;
  };
};

type AmapMap = {
  destroy(): void;
  on(name: string, listener: (event: AmapEvent) => void): void;
  panTo(position: [number, number]): void;
};

type AmapMarker = {
  on(name: string, listener: (event: AmapEvent) => void): void;
  setMap(map: AmapMap): void;
  setPosition(position: [number, number]): void;
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
        result: {
          info?: string;
          regeocode?: { formattedAddress?: string };
        },
      ) => void,
    ): void;
  };
  PlaceSearch: new (options?: { pageSize?: number }) => {
    search(
      keyword: string,
      callback: (
        status: string,
        result: {
          poiList?: {
            pois?: Array<{
              id?: string;
              name?: string;
              address?: string;
              location?: {
                getLat(): number;
                getLng(): number;
              };
            }>;
          };
        },
      ) => void,
    ): void;
  };
  convertFrom(
    position: [number, number],
    source: "gps",
    callback: (
      status: string,
      result: {
        info?: string;
        locations?: Array<{
          getLat(): number;
          getLng(): number;
        }>;
      },
    ) => void,
  ): void;
};

declare global {
  interface Window {
    _AMapSecurityConfig?: {
      serviceHost?: string;
      securityJsCode?: string;
    };
  }
}

type CandidateLocation = ConfirmedLocation & { address: string };
type PlaceSearchResult = {
  id: string;
  title: string;
  detail: string;
  longitude: number;
  latitude: number;
};

let amapPromise: Promise<AmapApi> | null = null;

function loadAmap(): Promise<AmapApi> {
  if (amapPromise) return amapPromise;

  const key = import.meta.env.VITE_AMAP_JS_API_KEY?.trim();
  if (!key) {
    return Promise.reject(
      new Error("地图服务尚未配置。你仍可手动填写地点。"),
    );
  }

  const serviceHost =
    import.meta.env.VITE_AMAP_JS_API_SERVICE_HOST?.trim() || "/_AMapService";
  const securityJsCode = import.meta.env.DEV
    ? import.meta.env.VITE_AMAP_JS_API_SECURITY_CODE?.trim()
    : undefined;

  window._AMapSecurityConfig = {
    serviceHost,
    ...(securityJsCode ? { securityJsCode } : {}),
  };

  amapPromise = loadAmapLibrary({
    key,
    version: "2.0",
    plugins: ["AMap.Geocoder", "AMap.PlaceSearch"],
  })
    .then((api) => api as AmapApi)
    .catch((error) => {
      amapPromise = null;
      throw error;
    });

  return amapPromise;
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

function precisionFor(accuracyMeters?: number): LocationPrecision {
  return accuracyMeters !== undefined && accuracyMeters <= 50
    ? "exact"
    : "approximate";
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
  const operationRef = useRef(0);
  const searchOperationRef = useRef(0);
  const [candidate, setCandidate] = useState<CandidateLocation | null>(null);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [isLocating, setIsLocating] = useState(false);
  const [isMapReady, setIsMapReady] = useState(false);
  const [isGeocoding, setIsGeocoding] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<PlaceSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [message, setMessage] = useState("");

  const resolveAddress = useCallback((longitude: number, latitude: number) => {
    const api = amap.current;
    const operation = operationRef.current;
    if (!api) return;

    setIsGeocoding(true);
    const geocoder = new api.Geocoder({ radius: 1_000, extensions: "base" });
    geocoder.getAddress([longitude, latitude], (status, result) => {
      if (operation !== operationRef.current) return;
      const address =
        status === "complete" && result.info === "OK"
          ? result.regeocode?.formattedAddress?.trim()
          : undefined;
      if (!address) {
        setIsGeocoding(false);
        setMessage("无法解析该点的文字地址。请改用手动填写地点。");
        setCandidate((current) =>
          current &&
          current.longitude === longitude &&
          current.latitude === latitude
            ? { ...current, address: "" }
            : current,
        );
        candidateRef.current = candidateRef.current
          ? { ...candidateRef.current, address: "" }
          : null;
        return;
      }

      setIsGeocoding(false);
      setMessage("");
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
  }, []);

  const chooseCoordinates = useCallback(
    (longitude: number, latitude: number) => {
      const current = candidateRef.current;
      if (!current) return;

      ++operationRef.current;
      const next: CandidateLocation = {
        longitude,
        latitude,
        address: "",
        accuracyMeters: current.accuracyMeters,
        precision: current.precision,
      };
      candidateRef.current = next;
      setCandidate(next);
      setMessage("");
      marker.current?.setPosition([longitude, latitude]);
      map.current?.panTo([longitude, latitude]);
      resolveAddress(longitude, latitude);
    },
    [resolveAddress],
  );

  const openPickerAtCoordinates = useCallback(
    (longitude: number, latitude: number) => {
      ++operationRef.current;
      const next: CandidateLocation = {
        longitude,
        latitude,
        address: "",
        precision: "approximate",
      };
      candidateRef.current = next;
      setCandidate(next);
      setIsPickerOpen(true);
      setMessage("");
      setSearchResults([]);
      marker.current?.setPosition([longitude, latitude]);
      map.current?.panTo([longitude, latitude]);
      if (map.current) resolveAddress(longitude, latitude);
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

    const operation = operationRef.current;
    let cancelled = false;
    setIsMapReady(false);
    void loadAmap()
      .then((api) => {
        if (
          cancelled ||
          operation !== operationRef.current ||
          !mapElement.current ||
          !candidateRef.current
        ) {
          return;
        }

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
        setIsMapReady(true);
        resolveAddress(initial.longitude, initial.latitude);
      })
      .catch((cause: unknown) => {
        if (!cancelled && operation === operationRef.current) {
          setIsMapReady(false);
          setMessage(
            cause instanceof Error
              ? cause.message
              : "地图服务暂不可用。你仍可手动填写地点。",
          );
        }
      });

    return () => {
      cancelled = true;
      operationRef.current += 1;
      map.current?.destroy();
      map.current = null;
      marker.current = null;
      amap.current = null;
      setIsMapReady(false);
      setIsGeocoding(false);
    };
  }, [chooseCoordinates, isPickerOpen, resolveAddress]);

  function requestLocation() {
    const operation = ++operationRef.current;
    setMessage("");
    setIsLocating(true);
    setIsMapReady(false);
    if (!navigator.geolocation) {
      setIsLocating(false);
      setMessage("此设备不支持系统定位。你仍可手动填写地点。");
      return;
    }

    navigator.geolocation.getCurrentPosition(
      ({ coords }) => {
        if (operation !== operationRef.current) return;
        void loadAmap()
          .then((api) => {
            if (operation !== operationRef.current) return;
            amap.current = api;
            api.convertFrom(
              [coords.longitude, coords.latitude],
              "gps",
              (status, result) => {
                if (operation !== operationRef.current) return;
                const point =
                  status === "complete" && result.info === "ok"
                    ? result.locations?.[0]
                    : undefined;
                if (!point) {
                  setIsLocating(false);
                  setMessage("无法完成坐标转换。你仍可手动填写地点。");
                  return;
                }

                const next: CandidateLocation = {
                  longitude: point.getLng(),
                  latitude: point.getLat(),
                  address: "",
                  accuracyMeters: coords.accuracy,
                  precision: precisionFor(coords.accuracy),
                };
                candidateRef.current = next;
                setCandidate(next);
                setIsPickerOpen(true);
                setIsLocating(false);
              },
            );
          })
          .catch((cause: unknown) => {
            if (operation !== operationRef.current) return;
            setIsLocating(false);
            setMessage(
              cause instanceof Error
                ? cause.message
                : "地图服务暂不可用。你仍可手动填写地点。",
            );
          });
      },
      (error) => {
        if (operation !== operationRef.current) return;
        setIsLocating(false);
        setMessage(browserLocationError(error));
      },
      { enableHighAccuracy: true, timeout: 12_000, maximumAge: 30_000 },
    );
  }

  function searchPlaces() {
    const query = searchQuery.trim();
    if (!query) {
      setMessage("请输入地点、地标或场所后再搜索。");
      return;
    }

    const operation = ++searchOperationRef.current;
    setMessage("");
    setSearchResults([]);
    setIsSearching(true);
    void loadAmap()
      .then((api) => {
        if (operation !== searchOperationRef.current) return;
        amap.current = api;
        const placeSearch = new api.PlaceSearch({ pageSize: 5 });
        placeSearch.search(query, (status, result) => {
          if (operation !== searchOperationRef.current) return;
          setIsSearching(false);
          const places =
            status === "complete"
              ? (result.poiList?.pois ?? []).flatMap((place, index) => {
                  const longitude = place.location?.getLng();
                  const latitude = place.location?.getLat();
                  if (longitude === undefined || latitude === undefined) {
                    return [];
                  }
                  return [
                    {
                      id: place.id ?? `${longitude},${latitude},${index}`,
                      title: place.name?.trim() || query,
                      detail: place.address?.trim() || "",
                      longitude,
                      latitude,
                    },
                  ];
                })
              : [];
          if (places.length === 0) {
            setMessage("未找到可定位的地点，请调整关键词后重试。");
            return;
          }
          setSearchResults(places);
        });
      })
      .catch((cause: unknown) => {
        if (operation !== searchOperationRef.current) return;
        setIsSearching(false);
        setMessage(
          cause instanceof Error
            ? cause.message
            : "地图服务暂不可用。你仍可手动填写地点。",
        );
      });
  }

  function clear() {
    operationRef.current += 1;
    searchOperationRef.current += 1;
    candidateRef.current = null;
    map.current?.destroy();
    map.current = null;
    marker.current = null;
    amap.current = null;
    setCandidate(null);
    setIsPickerOpen(false);
    setIsLocating(false);
    setIsMapReady(false);
    setIsGeocoding(false);
    setSearchQuery("");
    setSearchResults([]);
    setIsSearching(false);
    setMessage("");
    onClear();
  }

  const canConfirm =
    Boolean(candidate?.address) && isMapReady && !isGeocoding && !isLocating;

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
      <div className="flex gap-2">
        <input
          aria-label="搜索地点"
          value={searchQuery}
          maxLength={200}
          placeholder="输入地址、地标或场所"
          className="min-h-9 min-w-0 flex-1 rounded-md border border-slate-300 bg-white px-3 text-sm text-slate-900 outline-none focus:border-brand-600"
          onChange={(event) => setSearchQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              searchPlaces();
            }
          }}
        />
        <Button
          type="button"
          size="sm"
          variant="secondary"
          isDisabled={isSearching}
          onPress={searchPlaces}
        >
          <Search size={16} />
          {isSearching ? "正在搜索" : "搜索地点"}
        </Button>
      </div>
      {searchResults.length > 0 && (
        <ul className="m-0 grid list-none divide-y divide-slate-200 overflow-hidden rounded-md border border-slate-200 bg-white p-0">
          {searchResults.map((place) => (
            <li key={place.id}>
              <button
                type="button"
                className="block w-full px-3 py-2 text-left text-sm text-slate-800 hover:bg-brand-50 focus:bg-brand-50 focus:outline-none"
                onClick={() =>
                  openPickerAtCoordinates(place.longitude, place.latitude)
                }
              >
                <span className="block font-medium">{place.title}</span>
                {place.detail && (
                  <span className="mt-0.5 block text-xs text-slate-500">
                    {place.detail}
                  </span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
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
              <span>{candidate.address || "正在解析地点"}</span>
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
                isDisabled={!canConfirm}
                onPress={() => {
                  const current = candidateRef.current;
                  if (current && canConfirm) onConfirm(current);
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
