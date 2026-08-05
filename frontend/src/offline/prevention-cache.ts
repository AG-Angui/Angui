import { useSyncExternalStore } from "react";

let isReady = false;
const listeners = new Set<() => void>();

function setReady(nextReady: boolean) {
  if (isReady === nextReady) return;
  isReady = nextReady;
  listeners.forEach((listener) => listener());
}

export function registerPreventionCache() {
  if (!import.meta.env.PROD || !("serviceWorker" in navigator)) return;

  void navigator.serviceWorker
    .register("/sw.js")
    .then(() => navigator.serviceWorker.ready)
    .then(async () => {
      const cache = await caches.open("angui-learning-offline-v1");
      setReady(
        Boolean(navigator.serviceWorker.controller) &&
          Boolean(await cache.match("/")),
      );
    })
    .catch((cause: unknown) => {
      console.warn("防走失知识卡离线缓存未启用", cause);
      setReady(false);
    });
}

export function usePreventionCacheReady() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => isReady,
    () => false,
  );
}
