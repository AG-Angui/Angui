const CACHE_NAME = "angui-learning-offline-v1";
const PREVENTION_CARD_PATH = "/api/learning/public/prevention-card";

async function preCacheAppShell() {
  try {
    const cache = await caches.open(CACHE_NAME);
    const shell = await fetch("/", { cache: "reload" });
    if (!shell.ok) throw new Error(`app shell request failed with ${shell.status}`);
    const markup = await shell.clone().text();
    await cache.put("/", shell);
    const assetPaths = [...markup.matchAll(/(?:src|href)=["']([^"']+)["']/g)]
      .map((match) => new URL(match[1], self.location.origin))
      .filter((url) => url.origin === self.location.origin && url.pathname.startsWith("/assets/"));
    await Promise.all(assetPaths.map(async (url) => {
      try {
        const response = await fetch(url, { cache: "reload" });
        if (response.ok) await cache.put(url, response);
      } catch (cause) {
        console.warn("无法预缓存应用资源", url.pathname, cause);
      }
    }));
  } catch (cause) {
    console.warn("无法预缓存应用壳；将在联网时重试", cause);
  }
}

self.addEventListener("install", (event) => {
  event.waitUntil(preCacheAppShell());
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)))),
  );
  self.clients.claim();
});

async function networkFirst(request) {
  const cache = await caches.open(CACHE_NAME);
  try {
    const response = await fetch(request);
    if (response.status === 404 && new URL(request.url).pathname === PREVENTION_CARD_PATH) {
      await cache.delete(request);
    }
    if (response.ok) await cache.put(request, response.clone());
    return response;
  } catch {
    const cached = await cache.match(request);
    if (cached) return cached;
    throw new Error("offline response is unavailable");
  }
}

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  if (url.pathname === PREVENTION_CARD_PATH || request.mode === "navigate") {
    event.respondWith(networkFirst(request));
    return;
  }

  if (["script", "style", "image", "font"].includes(request.destination)) {
    event.respondWith(
      caches.open(CACHE_NAME).then(async (cache) => {
        const cached = await cache.match(request);
        if (cached) return cached;
        const response = await fetch(request);
        if (response.ok) await cache.put(request, response.clone());
        return response;
      }),
    );
  }
});
