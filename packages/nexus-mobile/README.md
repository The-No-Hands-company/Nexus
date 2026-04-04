# Nexus Mobile

Privacy-first Discord alternative — React Native / Expo client.

## Features

- **Auth** — Login and registration with configurable server URL
- **Real-time gateway** — WebSocket connection with Identify/Heartbeat/Dispatch,
  exponential backoff reconnect, MESSAGE_CREATE, MESSAGE_DELETE,
  PRESENCE_UPDATE, TYPING_START
- **Direct messages** — Full DM list, real-time message receive, load history
- **Servers** — Server list, channel list (text + voice sections)
- **Chat** — FlatList message view, grouped messages (same author within 5 min),
  message send, reply, delete (own messages), load older messages, typing indicator
- **Settings** — Profile card, gateway status, sign out

## Getting started

```bash
cd packages/nexus-mobile
npm install
npm run start          # Expo Go / development build
npm run android        # Android emulator or device
npm run ios            # iOS simulator or device
```

## Architecture

```
app/
  _layout.tsx                ← Root stack: auth guard + gateway connection
  (auth)/
    login.tsx                ← Login screen
    register.tsx             ← Register screen
  (app)/
    _layout.tsx              ← Bottom tab navigator (DMs / Servers / Settings)
    index.tsx                ← DM list
    servers.tsx              ← Server list
    settings.tsx             ← Profile + settings + logout
    server/[id].tsx          ← Channel list for a server
    channel/[id].tsx         ← Chat screen

src/
  types.ts                   ← Shared TypeScript types (Session, NxMessage, …)
  api.ts                     ← Typed fetch helpers (apiFetch, apiPost, …)
  store.ts                   ← Zustand store with AsyncStorage persistence
  gateway.ts                 ← WebSocket gateway hook
```

## Environment

The server URL is entered at login time — no build-time configuration needed.
Point it at any Nexus instance (`http://localhost:8080` for local dev).

## Planned

- Voice channel (WebRTC via expo-webrtc or a native bridge)
- Push notifications (expo-notifications + Nexus VAPID backend)
- File attachments (expo-image-picker + upload endpoint)
- E2EE channel support (Signal Protocol, on-device)
- i18n (i18next-react-native)
