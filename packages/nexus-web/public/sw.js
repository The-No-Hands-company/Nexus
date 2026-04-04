// Nexus Web Push Service Worker
//
// Handles push events from the server and shows browser notifications.
// Also handles notification click to focus/open the app.
//
// Installed by the main app via: navigator.serviceWorker.register('/sw.js')

const CACHE_NAME = 'nexus-sw-v1';

// ── Install ───────────────────────────────────────────────────────────────────

self.addEventListener('install', (event) => {
  // Activate immediately without waiting for the old SW to be replaced.
  self.skipWaiting();
});

// ── Activate ──────────────────────────────────────────────────────────────────

self.addEventListener('activate', (event) => {
  // Take control of all open clients immediately.
  event.waitUntil(self.clients.claim());
});

// ── Push ──────────────────────────────────────────────────────────────────────

self.addEventListener('push', (event) => {
  if (!event.data) return;

  let payload;
  try {
    payload = event.data.json();
  } catch {
    // Fallback for plain-text payloads
    payload = {
      title: 'Nexus',
      body: event.data.text(),
      icon: '/icon-192.png',
      url: '/',
    };
  }

  const {
    title = 'Nexus',
    body = '',
    icon = '/icon-192.png',
    badge = '/badge-72.png',
    url = '/',
    channel_id,
  } = payload;

  const options = {
    body,
    icon,
    badge,
    // Tag groups notifications from the same channel so they replace each other
    // rather than stacking (one notification per channel at a time).
    tag: channel_id ? `nexus-channel-${channel_id}` : 'nexus-general',
    renotify: true,
    // Carry the deep-link URL in the data field so the click handler can open it.
    data: { url },
    // Action buttons — "Open" is the primary CTA.
    actions: [
      { action: 'open', title: 'Open' },
    ],
    // Vibration pattern: short buzz
    vibrate: [200, 100, 200],
  };

  event.waitUntil(
    self.registration.showNotification(title, options)
  );
});

// ── Notification click ────────────────────────────────────────────────────────

self.addEventListener('notificationclick', (event) => {
  event.notification.close();

  const targetUrl = event.notification.data?.url || '/';

  event.waitUntil(
    self.clients
      .matchAll({ type: 'window', includeUncontrolled: true })
      .then((clients) => {
        // If the app is already open in a tab, focus it and navigate.
        for (const client of clients) {
          if ('focus' in client) {
            client.focus();
            if ('navigate' in client) {
              client.navigate(targetUrl);
            }
            return;
          }
        }
        // Otherwise open a new window/tab.
        if (self.clients.openWindow) {
          return self.clients.openWindow(targetUrl);
        }
      })
  );
});

// ── Push subscription change ──────────────────────────────────────────────────
// Fired when the push service rotates the endpoint (e.g. Firefox Autopush).
// We re-subscribe and send the new subscription to the server.

self.addEventListener('pushsubscriptionchange', (event) => {
  event.waitUntil(
    self.registration.pushManager
      .subscribe({
        userVisibleOnly: true,
        applicationServerKey: event.oldSubscription?.options?.applicationServerKey,
      })
      .then(async (newSub) => {
        // Post the new subscription to the app so it can relay it to the API.
        const clients = await self.clients.matchAll({ type: 'window' });
        for (const client of clients) {
          client.postMessage({
            type: 'PUSH_SUBSCRIPTION_CHANGED',
            subscription: newSub.toJSON(),
          });
        }
      })
  );
});
