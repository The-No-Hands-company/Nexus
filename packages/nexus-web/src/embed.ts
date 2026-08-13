/**
 * Whether this app is being rendered inside the ecosystem shell.
 *
 * A query parameter rather than postMessage or a header: it works identically
 * from any language and any framework, and an app that ignores it still
 * functions — just with its own chrome as well as the shell's. Degrading to
 * "slightly wrong" beats degrading to "blank".
 */
export function isEmbedded(search: string = window.location.search): boolean {
  return new URLSearchParams(search).get("embed") === "1";
}

export interface NotifBannerInput {
  embedded: boolean;
  dismissed: boolean;
  supported: boolean;
  hasSession: boolean;
  /**
   * Whether `Notification.permission === "default"`, i.e. the user has not
   * yet been asked. MainLayout's original condition checked this alongside
   * dismissed/support — it doesn't fit any of the other fields (it's neither
   * "unsupported" nor "dismissed": a "denied" or already-"granted" permission
   * is a real browser API state, not a UI dismissal), so it gets its own field
   * rather than being folded into one of those.
   */
  permissionDefault: boolean;
}

/**
 * Whether to offer the push-notification prompt.
 *
 * `embedded` is the non-obvious one. The shell frames chat.tnhc.dev from
 * app.tnhc.dev, which is cross-origin, and browsers refuse
 * Notification.requestPermission() in a cross-origin frame. Showing the banner
 * there means offering a button that cannot work — the click handler already
 * discards the failure silently, so the user gets no feedback at all.
 */
export function shouldShowNotifBanner(input: NotifBannerInput): boolean {
  if (input.embedded) return false;
  return (
    !input.dismissed &&
    input.supported &&
    input.hasSession &&
    input.permissionDefault
  );
}
