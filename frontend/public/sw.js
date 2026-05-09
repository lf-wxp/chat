/// Service Worker for WebRTC Chat PWA.
///
/// Provides offline support via cache-first strategy for static assets
/// and network-first strategy for API/WebSocket connections. Implements
/// a stale-while-revalidate pattern for HTML to balance freshness with
/// instant loading.

const CACHE_NAME = 'webrtc-chat-v2';

// Static assets to pre-cache during installation.
const PRECACHE_URLS = [
  '/',
  '/index.html',
  '/styles/main.css',
  '/styles/reset.css',
  '/styles/base.css',
  '/styles/tokens.css',
  '/styles/utilities.css',
  '/styles/glass.css',
  '/styles/animations.css',
  '/styles/background.css',
  '/manifest.json',
];

// Install event: pre-cache critical static assets.
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(PRECACHE_URLS);
    }).then(() => {
      return self.skipWaiting();
    })
  );
});

// Activate event: clean up old caches.
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames
          .filter((name) => name !== CACHE_NAME)
          .map((name) => caches.delete(name))
      );
    }).then(() => {
      return self.clients.claim();
    })
  );
});

// Fetch event: route requests to appropriate caching strategy.
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests (POST, PUT, etc.).
  if (request.method !== 'GET') {
    return;
  }

  // Skip WebSocket upgrade requests — these must go directly to the server.
  if (request.headers.get('upgrade') === 'websocket') {
    return;
  }

  // Skip cross-origin requests (STUN/TURN, external resources).
  if (url.origin !== self.location.origin) {
    return;
  }

  // Network-first for API calls (always try to get fresh data).
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(networkFirst(request));
    return;
  }

  // Cache-first for static assets (CSS, JS, WASM, images, fonts).
  if (isStaticAsset(url.pathname)) {
    event.respondWith(cacheFirst(request));
    return;
  }

  // Stale-while-revalidate for HTML navigation (instant load + background update).
  if (request.mode === 'navigate' || url.pathname.endsWith('.html')) {
    event.respondWith(staleWhileRevalidate(request));
    return;
  }

  // Default: network-first.
  event.respondWith(networkFirst(request));
});

/// Cache-first strategy: serve from cache, fall back to network.
/// Best for immutable static assets that rarely change.
async function cacheFirst(request) {
  const cached = await caches.match(request);
  if (cached) {
    return cached;
  }
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (_error) {
    return new Response('Offline', { status: 503, statusText: 'Service Unavailable' });
  }
}

/// Network-first strategy: try network, fall back to cache.
/// Best for API responses where freshness matters but offline support is desired.
async function networkFirst(request) {
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (_error) {
    const cached = await caches.match(request);
    if (cached) {
      return cached;
    }
    return new Response('Offline', { status: 503, statusText: 'Service Unavailable' });
  }
}

/// Stale-while-revalidate: respond from cache immediately, then update cache in background.
/// Best for HTML documents where instant loading matters but content should stay fresh.
async function staleWhileRevalidate(request) {
  const cache = await caches.open(CACHE_NAME);
  const cached = await cache.match(request);

  const fetchPromise = fetch(request).then((response) => {
    if (response.ok) {
      cache.put(request, response.clone());
    }
    return response;
  }).catch(() => cached);

  return cached || fetchPromise;
}

/// Check if a pathname refers to a static asset that can be safely cached.
///
/// Static assets include:
/// - CSS stylesheets, JavaScript, WebAssembly
/// - Images (png, jpg, gif, svg, ico, webp)
/// - Fonts (woff, woff2, ttf, eot)
/// - i18n locale JSON files (under /locales/)
/// - PWA manifest.json (pre-cached as well, but intercepted here too)
function isStaticAsset(pathname) {
  if (/\.(css|js|wasm|png|jpg|jpeg|gif|svg|ico|woff2?|ttf|eot|webp)$/i.test(pathname)) {
    return true;
  }
  // Locale files and the PWA manifest are effectively immutable per
  // deployment — safe to serve cache-first.
  if (pathname.startsWith('/locales/') && pathname.endsWith('.json')) {
    return true;
  }
  if (pathname === '/manifest.json') {
    return true;
  }
  return false;
}

/// Handle messages from the main thread.
self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }

  if (event.data && event.data.type === 'CLEAR_CACHE') {
    caches.delete(CACHE_NAME).then(() => {
      self.clients.matchAll().then((clients) => {
        clients.forEach((client) => {
          client.postMessage({ type: 'CACHE_CLEARED' });
        });
      });
    });
  }
});
