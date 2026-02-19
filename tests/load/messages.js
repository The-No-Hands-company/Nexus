/**
 * k6 load test — message send / receive
 *
 * Scenarios tested:
 *   - Create a guild + channel
 *   - Send messages at varying concurrency
 *   - Fetch message history
 *   - Edit a message
 *   - Delete a message
 *
 * Run:
 *   k6 run tests/load/messages.js
 *   k6 run --vus 20 --duration 60s tests/load/messages.js
 *
 * Environment variables:
 *   BASE_URL   - defaults to http://localhost:8080
 *   VUS        - virtual users (default: 10)
 *   DURATION   - test duration (default: 30s)
 *   TEST_TOKEN - pre-issued JWT (skips login if provided)
 */

import http from "k6/http";
import { check, sleep, group } from "k6";
import { Rate, Trend, Counter } from "k6/metrics";
import { uuidv4 } from "https://jslib.k6.io/k6-utils/1.4.0/index.js";

// ── Config ────────────────────────────────────────────────────────────────────

const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";

export const options = {
  stages: [
    { duration: "10s", target: Number(__ENV.VUS) || 10 },   // ramp-up
    { duration: __ENV.DURATION || "20s", target: Number(__ENV.VUS) || 10 }, // steady
    { duration: "5s", target: 0 },                          // ramp-down
  ],
  thresholds: {
    // 95th-percentile send under 300 ms, history fetch under 150 ms
    "http_req_duration{group:::send_message}": ["p(95)<300"],
    "http_req_duration{group:::fetch_history}": ["p(95)<150"],
    http_req_failed: ["rate<0.01"],
  },
};

// ── Custom metrics ────────────────────────────────────────────────────────────

const messagesSent = new Counter("messages_sent");
const sendDuration = new Trend("send_duration_ms", true);
const historyDuration = new Trend("history_duration_ms", true);

// ── Setup — runs once before VUs start ───────────────────────────────────────

export function setup() {
  // Use a pre-issued token or create a fresh one
  const preToken = __ENV.TEST_TOKEN;
  if (preToken) return { token: preToken };

  const username = `bench_setup_${uuidv4().replace(/-/g, "").slice(0, 8)}`;
  const password = "Bench3nch!Secure#2025";
  const email = `${username}@bench.invalid`;

  // Register
  http.post(
    `${BASE_URL}/api/v1/auth/register`,
    JSON.stringify({ username, password, email }),
    { headers: { "Content-Type": "application/json" } }
  );

  // Login
  const loginRes = http.post(
    `${BASE_URL}/api/v1/auth/login`,
    JSON.stringify({ username, password }),
    { headers: { "Content-Type": "application/json" } }
  );

  const token = JSON.parse(loginRes.body).access_token;

  // Create a guild
  const guildRes = http.post(
    `${BASE_URL}/api/v1/guilds`,
    JSON.stringify({ name: "Bench Guild" }),
    { headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` } }
  );
  const guildId = JSON.parse(guildRes.body).id;

  // Create a text channel
  const chanRes = http.post(
    `${BASE_URL}/api/v1/guilds/${guildId}/channels`,
    JSON.stringify({ name: "bench-general", kind: "text" }),
    { headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` } }
  );
  const channelId = JSON.parse(chanRes.body).id;

  return { token, guildId, channelId };
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function authHeaders(token) {
  return {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
  };
}

// ── Main scenario ─────────────────────────────────────────────────────────────

export default function (data) {
  const { token, channelId } = data;
  if (!token || !channelId) {
    console.warn("setup data missing — skipping iteration");
    sleep(1);
    return;
  }

  // ── Send a message ──────────────────────────────────────────────────────────
  let messageId = null;
  group("send_message", () => {
    const payload = JSON.stringify({
      content: `Bench message ${uuidv4()} at ${Date.now()}`,
    });
    const res = http.post(
      `${BASE_URL}/api/v1/channels/${channelId}/messages`,
      payload,
      authHeaders(token)
    );

    sendDuration.add(res.timings.duration);
    messagesSent.add(1);

    check(res, {
      "send: status 201": (r) => r.status === 201,
      "send: has id": (r) => {
        try {
          messageId = JSON.parse(r.body).id;
          return !!messageId;
        } catch {
          return false;
        }
      },
    });
  });

  sleep(0.1);

  // ── Fetch message history ───────────────────────────────────────────────────
  group("fetch_history", () => {
    const res = http.get(
      `${BASE_URL}/api/v1/channels/${channelId}/messages?limit=50`,
      authHeaders(token)
    );

    historyDuration.add(res.timings.duration);

    check(res, {
      "history: status 200": (r) => r.status === 200,
      "history: is array": (r) => {
        try {
          return Array.isArray(JSON.parse(r.body));
        } catch {
          return false;
        }
      },
    });
  });

  sleep(0.1);

  // ── Edit the message ────────────────────────────────────────────────────────
  if (messageId) {
    group("edit_message", () => {
      const res = http.patch(
        `${BASE_URL}/api/v1/channels/${channelId}/messages/${messageId}`,
        JSON.stringify({ content: "Edited bench message" }),
        authHeaders(token)
      );

      check(res, {
        "edit: status 200": (r) => r.status === 200,
      });
    });

    sleep(0.1);

    // ── Delete the message ────────────────────────────────────────────────────
    group("delete_message", () => {
      const res = http.del(
        `${BASE_URL}/api/v1/channels/${channelId}/messages/${messageId}`,
        null,
        authHeaders(token)
      );

      check(res, {
        "delete: status 204": (r) => r.status === 204,
      });
    });
  }

  sleep(0.5);
}
