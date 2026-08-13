import { describe, it, expect } from "vitest";
import { isEmbedded, shouldShowNotifBanner } from "./embed";

describe("isEmbedded", () => {
  it("is true when the shell asks for it", () => {
    expect(isEmbedded("?embed=1")).toBe(true);
  });

  it("is false normally", () => {
    expect(isEmbedded("")).toBe(false);
  });

  it("ignores other values, so ?embed=0 does not embed", () => {
    expect(isEmbedded("?embed=0")).toBe(false);
  });

  it("survives other parameters alongside it", () => {
    expect(isEmbedded("?channel=7&embed=1")).toBe(true);
  });
});

describe("shouldShowNotifBanner", () => {
  const base = {
    embedded: false,
    dismissed: false,
    supported: true,
    hasSession: true,
    permissionDefault: true,
  };

  it("shows for a signed-in user on a supporting browser", () => {
    expect(shouldShowNotifBanner(base)).toBe(true);
  });

  it("stays hidden once dismissed", () => {
    expect(shouldShowNotifBanner({ ...base, dismissed: true })).toBe(false);
  });

  it("stays hidden where the browser cannot do push", () => {
    expect(shouldShowNotifBanner({ ...base, supported: false })).toBe(false);
  });

  it("stays hidden with no session", () => {
    expect(shouldShowNotifBanner({ ...base, hasSession: false })).toBe(false);
  });

  it("stays hidden when embedded, because a cross-origin frame cannot grant permission", () => {
    // The one this task exists for. Without it the banner offers a button
    // whose handler is already writing its own failure off as `catch {}`.
    expect(shouldShowNotifBanner({ ...base, embedded: true })).toBe(false);
  });

  it("stays hidden once the user has already granted or denied permission", () => {
    // The original MainLayout expression checked `Notification.permission
    // === "default"` alongside the dismissed/support checks. That condition
    // doesn't map onto embedded/dismissed/supported/hasSession, so it gets
    // its own field rather than being folded into one of those.
    expect(shouldShowNotifBanner({ ...base, permissionDefault: false })).toBe(false);
  });
});
