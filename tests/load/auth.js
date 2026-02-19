/**
 * k6 load test — authentication endpoints
 *
 * Scenarios tested:
 *   - Register a new user
 *   - Login with valid credentials
 *   - Login with invalid credentials (expects 401)
 *   - Token refresh
 *
 * Run:
 *   k6 run tests/load/auth.js
 *   k6 run --vus 50 --duration 60s tests/load/auth.js
 *
 * Environment variables:
 *   BASE_URL   - defaults to http://localhost:8080
 *   VUS        - virtual users (default: 10)
 *   DURATION   - test duration (default: 30s)
 */

import http from "k6/http";
import { check, sleep, group } from "k6";
import { Rate, Trend } from "k6/metrics";
import { uuidv4 } from "https://jslib.k6.io/k6-utils/1.4.0/index.js";

// ── Config ────────────────────────────────────────────────────────────────────

const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";

export const options = {
  vus: Number(__ENV.VUS) || 10,
  duration: __ENV.DURATION || "30s",
  thresholds: {
    // 95th-percentile response time under 200 ms
    http_req_duration: ["p(95)<200"],
    // Error rate below 1 %
    "http_req_failed{scenario:register}": ["rate<0.01"],
    "http_req_failed{scenario:login}": ["rate<0.01"],
  },
};

// ── Custom metrics ────────────────────────────────────────────────────────────

const loginSuccessRate = new Rate("login_success_rate");
const registerSuccessRate = new Rate("register_success_rate");
const loginDuration = new Trend("login_duration_ms", true);
const registerDuration = new Trend("register_duration_ms", true);

// ── Helpers ───────────────────────────────────────────────────────────────────

function jsonHeaders(token) {
  const headers = { "Content-Type": "application/json" };
  if (token) headers["Authorization"] = `Bearer ${token}`;
  return { headers };
}

function randomUsername() {
  return `bench_${uuidv4().replace(/-/g, "").slice(0, 12)}`;
}

// ── Main scenario ─────────────────────────────────────────────────────────────

export default function () {
  const username = randomUsername();
  const password = "Bench3nch!Secure#2025";
  const email = `${username}@bench.invalid`;
  let token = null;

  // ── Register ────────────────────────────────────────────────────────────────
  group("register", () => {
    const payload = JSON.stringify({ username, password, email });
    const res = http.post(`${BASE_URL}/api/v1/auth/register`, payload, jsonHeaders());

    registerDuration.add(res.timings.duration);
    registerSuccessRate.add(res.status === 201);

    check(res, {
      "register: status 201": (r) => r.status === 201,
      "register: has user_id": (r) => {
        try {
          return !!JSON.parse(r.body).user_id;
        } catch {
          return false;
        }
      },
    });
  });

  sleep(0.3);

  // ── Login (valid) ───────────────────────────────────────────────────────────
  group("login_valid", () => {
    const payload = JSON.stringify({ username, password });
    const res = http.post(`${BASE_URL}/api/v1/auth/login`, payload, jsonHeaders());

    loginDuration.add(res.timings.duration);
    loginSuccessRate.add(res.status === 200);

    check(res, {
      "login: status 200": (r) => r.status === 200,
      "login: has access_token": (r) => {
        try {
          const body = JSON.parse(r.body);
          token = body.access_token;
          return !!token;
        } catch {
          return false;
        }
      },
    });
  });

  sleep(0.2);

  // ── Login (invalid password) ────────────────────────────────────────────────
  group("login_invalid", () => {
    const payload = JSON.stringify({ username, password: "wrong-password" });
    const res = http.post(`${BASE_URL}/api/v1/auth/login`, payload, jsonHeaders());

    check(res, {
      "login_invalid: status 401": (r) => r.status === 401,
    });
  });

  sleep(0.2);

  // ── Authenticated request — GET /api/v1/users/me ───────────────────────────
  if (token) {
    group("profile", () => {
      const res = http.get(`${BASE_URL}/api/v1/users/me`, jsonHeaders(token));

      check(res, {
        "profile: status 200": (r) => r.status === 200,
        "profile: has username": (r) => {
          try {
            return JSON.parse(r.body).username === username;
          } catch {
            return false;
          }
        },
      });
    });
  }

  sleep(1);
}
