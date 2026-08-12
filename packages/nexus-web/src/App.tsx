import { useEffect, useState } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useStore, bootstrapSession } from "./store";

/**
 * Where to send someone who is not signed in.
 *
 * Straight to the ecosystem sign-in, carrying this page as the return address
 * so they come back to what they were trying to open. Reloading instead — which
 * this used to do — only works if the proxy is going to redirect, and when the
 * bootstrap failed for any other reason it looked to the user like a button
 * that did nothing.
 */
const AUTH_LOGIN_URL =
  import.meta.env.VITE_AUTH_LOGIN_URL ?? "https://auth.tnhc.dev/login";

function signInHref(): string {
  return `${AUTH_LOGIN_URL}?redirect_uri=${encodeURIComponent(window.location.href)}`;
}
import MainLayout from "./pages/MainLayout";
import { ErrorBoundary } from "./components/ErrorBoundary";

/**
 * There is no login screen any more.
 *
 * Accounts live in Nexus-Auth and the ecosystem proxy authenticates the browser
 * before this app is ever served, so by the time this code runs the user is
 * either already signed in or the proxy has redirected them to auth.tnhc.dev.
 * Asking again here would be a second login for one account — the exact thing
 * single sign-on exists to remove.
 *
 * So the app's only startup question is "who am I", answered by the server.
 */
export default function App() {
  const { session, setSession } = useStore();
  // A session persisted before the SSO cutover carries an accessToken, which
  // nothing issues or accepts any more. Treat it as absent and re-check with
  // the server, rather than rendering the app around a credential that cannot
  // work.
  const stale = !!session && session.accessToken !== "";
  const usable = session && !stale ? session : null;
  const [checking, setChecking] = useState(!usable);

  useEffect(() => {
    if (usable) return;
    let cancelled = false;
    void bootstrapSession().then((s) => {
      if (cancelled) return;
      setSession(s);
      setChecking(false);
    });
    return () => {
      cancelled = true;
    };
  }, [usable, setSession]);

  if (checking) {
    return <div className="flex h-screen items-center justify-center">Signing you in…</div>;
  }

  if (!usable) {
    return (
      <div className="flex h-screen flex-col items-center justify-center gap-4">
        <p>Not signed in.</p>
        <a className="rounded bg-blue-600 px-4 py-2 text-white" href={signInHref()}>
          Sign in
        </a>
      </div>
    );
  }

  return (
    <ErrorBoundary>
      <Routes>
        {/* Kept as redirects so old bookmarks and in-app links do not 404. */}
        <Route path="/login" element={<Navigate to="/" replace />} />
        <Route path="/register" element={<Navigate to="/" replace />} />
        <Route path="/*" element={<MainLayout />} />
      </Routes>
    </ErrorBoundary>
  );
}
