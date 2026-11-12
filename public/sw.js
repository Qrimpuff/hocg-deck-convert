const CACHE_NAME = 'hocg-deck-convert-v1.7.1';
// Cache Storage is shared by the whole origin, so only this prefix is owned by the app.
const APP_CACHE_PREFIX = 'hocg-deck-convert-';
const BASE_PATH = '/hocg-deck-convert';
let offlineHint = false;

// Keep prefetched card images across app updates.
const CARD_CACHE_NAME = 'hocg-deck-convert-cards';
const IMAGE_BASE_JP = 'https://qrimpuff.github.io/hocg-fan-sim-assets/img/';
const IMAGE_BASE_EN = 'https://qrimpuff.github.io/hocg-fan-sim-assets/img_en/';

// Keep price data across app updates.
const PRICE_CACHE_NAME = 'hocg-deck-convert-prices';
const PRICE_BASE_URL = 'https://qrimpuff.github.io/hocg-fan-sim-prices/';

const CARDS_DB_URL = 'https://qrimpuff.github.io/hocg-fan-sim-assets/hocg_cards.json';

// Static assets that should always be available offline.
const ASSETS = [
  BASE_PATH + '/',
  BASE_PATH + '/index.html',
  BASE_PATH + '/assets/android-chrome-192x192.png',
  BASE_PATH + '/assets/android-chrome-512x512.png',
  BASE_PATH + '/assets/apple-touch-icon.png',
  BASE_PATH + '/assets/arts_blue.png',
  BASE_PATH + '/assets/arts_green.png',
  BASE_PATH + '/assets/arts_null.png',
  BASE_PATH + '/assets/arts_purple.png',
  BASE_PATH + '/assets/arts_red.png',
  BASE_PATH + '/assets/arts_white.png',
  BASE_PATH + '/assets/arts_yellow.png',
  BASE_PATH + '/assets/card-back.webp',
  BASE_PATH + '/assets/cheer-back.webp',
  BASE_PATH + '/assets/favicon-16x16.png',
  BASE_PATH + '/assets/favicon-32x32.png',
  BASE_PATH + '/assets/image.png',
  BASE_PATH + '/assets/price_history.js',
  BASE_PATH + '/assets/site.webmanifest',
  BASE_PATH + '/assets/style.css',
  BASE_PATH + '/assets/untrack.html',
  BASE_PATH + '/favicon.ico'
];

const INDEX_URL = BASE_PATH + '/index.html';

const NETWORK_TIMEOUT_MS = 5000;

function fetchWithTimeout(request, timeoutMs = NETWORK_TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Network timeout')), timeoutMs);
    fetch(request).then(
      (response) => {
        clearTimeout(timer);
        resolve(response);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      }
    );
  });
}

function buildPrefetchRequest(url) {
  try {
    const resolved = new URL(url, self.location.origin);
    return new Request(resolved.toString(), {
      mode: 'cors',
      credentials: 'omit',
      cache: 'no-store'
    });
  } catch (error) {
    console.warn('[SW] Invalid prefetch URL', url, error);
    return null;
  }
}

const PREFETCH_MAX_ATTEMPTS = 5;

async function fetchPrefetchWithRetry(url) {
  for (let attempt = 1; ; attempt++) {
    const request = buildPrefetchRequest(url);
    if (!request) return null;

    const response = await fetch(request);
    const throttled = response.status === 429 || response.status === 503;
    if (!throttled || attempt >= PREFETCH_MAX_ATTEMPTS) return response;

    const retryAfter = parseInt(response.headers.get('Retry-After'), 10);
    const delayMs = Number.isFinite(retryAfter) ? retryAfter * 1000 : 1000 * 2 ** attempt;
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
}

function normalizeAssetUrl(url) {
  if (!url) return null;
  if (url.startsWith('http://') || url.startsWith('https://')) return url;
  if (url.startsWith('/')) return url;
  return BASE_PATH + '/' + url.replace(/^\.\//, '');
}

function extractHashedAssets(htmlText) {
  const urls = new Set();
  const patterns = [
    /<script[^>]+src="([^"]+)"/gi,
    /<link[^>]+href="([^"]+)"/gi
  ];

  patterns.forEach((re) => {
    let match;
    while ((match = re.exec(htmlText))) {
      const url = normalizeAssetUrl(match[1]);
      if (url) urls.add(url);
    }
  });

  return Array.from(urls).filter((url) => {
    const isAppAsset = typeof url === 'string' && url.startsWith(BASE_PATH + '/');
    if (!isAppAsset) return false;
    return url.endsWith('.js') || url.endsWith('.wasm') || url.endsWith('.css');
  });
}

function extractWasmAssetsFromJs(jsText) {
  if (!jsText) return [];

  const urls = new Set();
  const wasmStringRe = /["'`]([^"'`\s]+\.wasm(?:\?[^"'`\s]+)?)['"`]/g;
  let match;
  while ((match = wasmStringRe.exec(jsText))) {
    const candidate = match[1];
    if (!candidate) continue;
    if (candidate.startsWith('data:')) continue;

    const normalized = normalizeAssetUrl(candidate);
    // Only cache deployable WASM assets shipped with the app.
    if (normalized && normalized.startsWith(BASE_PATH + '/assets/')) {
      urls.add(normalized);
    }
  }

  return Array.from(urls);
}

function extractExternalStylesheetLinks(htmlText) {
  if (!htmlText) return [];
  const urls = new Set();

  // Capture: <link rel="stylesheet" ... href="https://...">
  const re = /<link\s+[^>]*rel=["']stylesheet["'][^>]*href=["']([^"']+)["'][^>]*>/gi;
  let match;
  while ((match = re.exec(htmlText))) {
    const href = (match[1] || '').trim();
    if (!href) continue;
    if (href.startsWith('http://') || href.startsWith('https://')) {
      urls.add(href);
    }
  }

  return Array.from(urls);
}

function extractExternalScriptLinks(htmlText) {
  if (!htmlText) return [];
  const urls = new Set();

  const re = /<script\s+[^>]*src=["']([^"']+)["'][^>]*>/gi;
  let match;
  while ((match = re.exec(htmlText))) {
    const src = (match[1] || '').trim();
    if (src.startsWith('http://') || src.startsWith('https://')) {
      urls.add(src);
    }
  }

  return Array.from(urls);
}

async function cacheAddAllBestEffort(cache, urls) {
  for (const url of urls) {
    try {
      await cache.add(url);
    } catch (error) {
      console.warn('[SW] Failed to pre-cache', url, error);
    }
  }
}

function extractUrlsFromCss(cssText, baseUrl) {
  if (!cssText) return [];
  const urls = new Set();

  // Very small CSS url(...) extractor.
  // Handles: url(x), url('x'), url("x")
  const re = /url\(\s*(['"]?)([^'"\)]+)\1\s*\)/g;
  let match;
  while ((match = re.exec(cssText))) {
    const raw = (match[2] || '').trim();
    if (!raw) continue;
    if (raw.startsWith('data:')) continue;

    try {
      const resolved = new URL(raw, baseUrl).toString();
      urls.add(resolved);
    } catch (_) {
      // ignore
    }
  }

  return Array.from(urls);
}

async function fetchAndCacheTextAsset(cache, url) {
  const response = await fetch(url, { cache: 'no-store' });
  if (!response || !(response.ok || response.type === 'opaque')) {
    throw new Error('Failed to fetch ' + url + ' (' + (response ? response.status : 'no response') + ')');
  }

  // Cache the response first; we'll read from the clone.
  await cache.put(url, response.clone());

  // Opaque responses can't be read as text; still useful to cache, but we can't extract url(...).
  if (response.type === 'opaque') {
    return null;
  }

  return await response.text();
}

async function matchFromKnownCaches(request) {
  try {
    const appCache = await caches.open(CACHE_NAME);
    const cached = await appCache.match(request, { ignoreVary: true });
    if (cached) return cached;
  } catch (_) {
    // ignore
  }

  try {
    const cardCache = await caches.open(CARD_CACHE_NAME);
    const cached = await cardCache.match(request, { ignoreVary: true });
    if (cached) return cached;
  } catch (_) {
    // ignore
  }

  try {
    const priceCache = await caches.open(PRICE_CACHE_NAME);
    return await priceCache.match(request, { ignoreVary: true });
  } catch (_) {
    return undefined;
  }
}

// Card images checked against the network while this worker runs, so each one is checked
// at most once until the worker restarts.
const revalidatedCardImages = new Set();

function corsImageRequest(url, init = {}) {
  return new Request(url, { mode: 'cors', credentials: 'omit', ...init });
}

// Opened once per worker, instead of once per image.
let cardCachePromise = null;
function openCardCache() {
  if (!cardCachePromise) {
    cardCachePromise = caches.open(CARD_CACHE_NAME).catch((error) => {
      cardCachePromise = null;
      throw error;
    });
  }
  return cardCachePromise;
}

async function matchCardImage(request) {
  try {
    const cardCache = await openCardCache();
    return await cardCache.match(request.url, { ignoreVary: true });
  } catch (_) {
    return undefined;
  }
}

// Size of a cached image, from its header when possible to avoid reading the image.
async function cachedImageSize(response) {
  if (response.type === 'opaque') return 0;
  const size = parseInt(response.headers.get('content-length'), 10);
  if (Number.isFinite(size)) return size;
  return (await response.clone().blob()).size;
}

// Lets the page finish loading its images before the background checks start.
const REVALIDATE_DELAY_MS = 3000;

async function sendUpdatedCardImage(url, response) {
  const blob = await response.blob();
  const clients = await self.clients.matchAll({ type: 'window' });
  for (const client of clients) {
    client.postMessage({ type: 'CARD_IMAGE_UPDATED', url, blob });
  }
}

// Checks if the image changed and, if so, updates the cache and sends the new image to the
// pages showing it.
//
// GitHub Pages does not expose the ETag to CORS requests, and its Last-Modified is the deploy
// time of the whole site, so the size is the only cheap signal: a HEAD request compares it,
// and the image is downloaded only when it differs. An update keeping the exact same size is
// missed until the next offline download.
async function revalidateCardImage(url, cached) {
  if (revalidatedCardImages.has(url)) return;
  revalidatedCardImages.add(url);

  try {
    await new Promise((resolve) => setTimeout(resolve, REVALIDATE_DELAY_MS));

    // An opaque copy has no readable size; replace it without swapping the image, since it
    // is most likely the same.
    const cachedSize = await cachedImageSize(cached);
    if (cachedSize > 0) {
      const head = await fetch(corsImageRequest(url, { method: 'HEAD', cache: 'no-store' }));
      if (!head.ok) {
        revalidatedCardImages.delete(url);
        return;
      }
      const size = parseInt(head.headers.get('content-length'), 10);
      if (!Number.isFinite(size) || size === cachedSize) return;
    }

    const response = await fetch(corsImageRequest(url, { cache: 'no-store' }));
    if (!response.ok) {
      revalidatedCardImages.delete(url);
      return;
    }

    const cardCache = await openCardCache();
    await cardCache.put(url, response.clone());
    if (cachedSize > 0) {
      await sendUpdatedCardImage(url, response);
    }
  } catch (error) {
    // Allow another try the next time the image is shown.
    revalidatedCardImages.delete(url);
    console.warn('[SW] Failed to revalidate card image', url, error);
  }
}

// Stale-while-revalidate: serve the cached image right away and check for a newer one in the
// background. Images not cached yet come from the network and are kept for next time.
async function serveCardImage(event) {
  const url = event.request.url;
  const cached = await matchCardImage(event.request);
  if (cached) {
    event.waitUntil(revalidateCardImage(url, cached));
    return cached;
  }

  // Download with CORS instead of the opaque <img> request: the response can be checked,
  // and opaque responses take far more storage quota than their size.
  const response = await fetch(corsImageRequest(url));
  if (response.ok) {
    // Just downloaded, no need to check it again.
    revalidatedCardImages.add(url);
    const responseToCache = response.clone();
    event.waitUntil(
      openCardCache()
        .then((cache) => cache.put(url, responseToCache))
        .catch((error) => console.warn('[SW] Failed to cache card image', url, error))
    );
  }
  return response;
}

self.addEventListener('install', (event) => {
  self.skipWaiting();
  event.waitUntil(
    caches.open(CACHE_NAME).then(async (cache) => {
      console.log('[SW] Pre-caching assets');
      await cacheAddAllBestEffort(cache, ASSETS);

      // Parse index.html to capture hashed build outputs (js/wasm/css).
      try {
        const response = await fetch(INDEX_URL, { cache: 'no-store' });
        const htmlText = await response.text();
        const hashedAssets = extractHashedAssets(htmlText);

        await cacheAddAllBestEffort(cache, hashedAssets);

        // Dioxus' entry JS often references the real hashed WASM dynamically.
        // Scan the cached JS bundles for `.wasm` URLs and pre-cache them too.
        const jsAssets = hashedAssets.filter((url) => url.endsWith('.js') && url.startsWith(BASE_PATH + '/'));
        const wasmAssets = new Set();
        for (const jsUrl of jsAssets) {
          try {
            const cachedJs = await cache.match(jsUrl, { ignoreVary: true });
            if (!cachedJs) continue;
            const jsText = await cachedJs.clone().text();
            for (const wasmUrl of extractWasmAssetsFromJs(jsText)) {
              wasmAssets.add(wasmUrl);
            }
          } catch (error) {
            console.warn('[SW] Failed to scan JS for WASM', jsUrl, error);
          }
        }

        if (wasmAssets.size > 0) {
          await cacheAddAllBestEffort(cache, Array.from(wasmAssets));
        }

        await cacheAddAllBestEffort(cache, extractExternalScriptLinks(htmlText));

        // Best-effort cache of cross-origin cards DB (avoid failing SW install if offline/CORS).
        await cacheAddAllBestEffort(cache, [CARDS_DB_URL]);

        // Best-effort cache of any external stylesheets (and their referenced assets) from index.html.
        try {
          const externalCssUrls = extractExternalStylesheetLinks(htmlText);
          for (const cssUrl of externalCssUrls) {
            try {
              const cssText = await fetchAndCacheTextAsset(cache, cssUrl);
              if (!cssText) continue;
              const referenced = extractUrlsFromCss(cssText, cssUrl);
              await cacheAddAllBestEffort(cache, referenced);
            } catch (error) {
              console.warn('[SW] External CSS pre-cache skipped', cssUrl, error);
            }
          }
        } catch (error) {
          console.warn('[SW] External CSS pre-cache skipped', error);
        }
      } catch (error) {
        console.warn('[SW] Failed to pre-cache hashed assets', error);
      }
    })
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cacheName) => {
          if (cacheName.startsWith(APP_CACHE_PREFIX) && cacheName !== CACHE_NAME && cacheName !== CARD_CACHE_NAME && cacheName !== PRICE_CACHE_NAME) {
            return caches.delete(cacheName);
          }
        })
      );
    }).then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  // Only cache GET requests
  if (event.request.method !== 'GET') {
    return;
  }

  // If the app is offline, serve directly from cache.
  if (offlineHint) {
    event.respondWith(
      matchFromKnownCaches(event.request).then((cached) => {
        if (cached) return cached;
        return fetch(event.request);
      })
    );
    return;
  }

  // Navigation requests
  // Keeps the offline shell current without relying on a CACHE_NAME bump.
  if (event.request.mode === 'navigate') {
    event.respondWith(
      fetchWithTimeout(event.request).then((networkResponse) => {
        if (networkResponse && networkResponse.ok) {
          const responseToCache = networkResponse.clone();
          event.waitUntil(
            caches.open(CACHE_NAME).then((cache) => cache.put(INDEX_URL, responseToCache))
          );
        }
        return networkResponse;
      }).catch(() => {
        return caches.match(INDEX_URL).then((cached) => cached || caches.match(BASE_PATH + '/'));
      })
    );
    return;
  }

  // General Strategy for other requests: Network First
  // 1. Fetch from Network
  // 2. If successful, update cache and return
  // 3. If network fails, return from Cache
  const url = event.request.url || '';
  const isCardImage = url.startsWith(IMAGE_BASE_JP) || url.startsWith(IMAGE_BASE_EN);
  const isPrice = url.startsWith(PRICE_BASE_URL);

  if (isCardImage) {
    event.respondWith(serveCardImage(event));
    return;
  }

  const networkFetch = fetchWithTimeout(event.request);

  event.respondWith(
    networkFetch.then((networkResponse) => {
      // Card images are cached by serveCardImage and PREFETCH_CARDS, which prunes stale entries.
      if (isPrice && networkResponse && networkResponse.ok) {
        const responseToCache = networkResponse.clone();
        event.waitUntil(
          caches.open(PRICE_CACHE_NAME).then((cache) => cache.put(event.request, responseToCache))
        );
      } else if (!isPrice && !isCardImage && networkResponse && (networkResponse.ok || networkResponse.type === 'opaque')) {
        const responseToCache = networkResponse.clone();
        event.waitUntil(
          caches.open(CACHE_NAME).then((cache) => cache.put(event.request, responseToCache))
        );
      }
      return networkResponse;
    }).catch(() => {
      // Network failed, look in cache
      return matchFromKnownCaches(event.request);
    })
  );
});

// Prefetch large card image sets on demand.
self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'OFFLINE_HINT') {
    offlineHint = !!event.data.offline;
    return;
  }

  if (event.data && event.data.type === 'PREFETCH_CARDS') {
    const urls = event.data.urls;
    const client = event.source;
    console.log('[SW] Prefetching ' + urls.length + ' cards...');

    event.waitUntil(
      caches.open(CARD_CACHE_NAME).then(async (cache) => {
        // Keep card cache aligned with the latest card DB list.
        const expectedUrls = new Set(urls);
        // Images already cached, from a single listing of the cache: matching each URL instead
        // takes thousands of Cache API calls on every page load once the download is done.
        let cachedUrls = null;
        try {
          const cachedRequests = await cache.keys();
          cachedUrls = new Set();
          await Promise.all(cachedRequests.map(async (request) => {
            const url = request.url || '';
            const isCardImage = url.startsWith(IMAGE_BASE_JP) || url.startsWith(IMAGE_BASE_EN);
            if (!isCardImage) return;
            if (expectedUrls.has(url)) {
              cachedUrls.add(url);
              return;
            }
            await cache.delete(request, { ignoreVary: true });
          }));
        } catch (error) {
          cachedUrls = null;
          console.warn('[SW] Failed pruning stale cards', error);
        }

        // Prefer replying directly to the requesting client. If not available, broadcast.
        const progressTargets = [];
        if (client && typeof client.postMessage === 'function') {
          progressTargets.push(client);
        } else {
          try {
            const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
            progressTargets.push(...clients);
          } catch (_) {
            // ignore
          }
        }

        // Skip already cached URLs.
        const urlsToFetch = [];
        for (const url of urls) {
          if (cachedUrls) {
            if (cachedUrls.has(url)) continue;
          } else if (await cache.match(url, { ignoreVary: true })) {
            continue;
          }
          urlsToFetch.push(url);
        }

        let completed = 0;
        let totalBytes = 0;

        // Rolling worker pool.
        const CONCURRENCY = 4;
        const PROGRESS_THROTTLE_MS = 16; // 60 fps
        let lastProgressAt = 0;

        const sendProgress = (delta = 0, force = false) => {
          completed += delta;
          if (progressTargets.length === 0) return;

          const now = Date.now();
          if (!force && now - lastProgressAt < PROGRESS_THROTTLE_MS) return;
          lastProgressAt = now;

          const message = {
            type: 'PREFETCH_PROGRESS',
            loaded: Math.min(completed, urlsToFetch.length),
            total: urlsToFetch.length,
            size: totalBytes
          };

          for (const target of progressTargets) {
            try {
              target.postMessage(message);
            } catch (_) {
              // ignore
            }
          }
        };

        // Notify start
        sendProgress(0, true);

        console.log('[SW] ' + urlsToFetch.length + ' cards to download, ' + (urls.length - urlsToFetch.length) + ' already cached');

        const workerCount = Math.max(1, Math.min(CONCURRENCY, urlsToFetch.length));

        const worker = async (workerId) => {
          for (let idx = workerId; idx < urlsToFetch.length; idx += workerCount) {
            const url = urlsToFetch[idx];
            try {
              const cached = await cache.match(url, { ignoreVary: true });
              if (cached) {
                continue;
              }

              const response = await fetchPrefetchWithRetry(url);
              if (response && (response.ok || response.type === 'opaque')) {
                const len = response.headers.get('content-length');
                if (len) totalBytes += parseInt(len, 10);
                await cache.put(url, response);
              }
            } catch (error) {
              console.error(error);
            } finally {
              sendProgress(1);
            }
          }
        };

        await Promise.all(Array.from({ length: workerCount }, (_v, i) => worker(i)));

        // Final progress update
        sendProgress(0, true);
      })
    );
  }
});
