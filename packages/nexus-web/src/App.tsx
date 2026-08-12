import { useEffect, useState } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useStore, bootstrapSession } from "./store";
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
  const [checking, setChecking] = useState(!session);

  useEffect(() => {
    if (session) return;
    let cancelled = false;
    void bootstrapSession().then((s) => {
      if (cancelled) return;
      if (s) setSession(s);
      setChecking(false);
    });
    return () => {
      cancelled = true;
    };
  }, [session, setSession]);

  if (checking) {
    return <div className="flex h-screen items-center justify-center">Signing you in…</div>;
  }

  if (!session) {
    // The proxy should have caught this. Reloading hands the request back to
    // it, which redirects to the ecosystem sign-in rather than showing a form
    // this app no longer owns.
    return (
      <div className="flex h-screen flex-col items-center justify-center gap-4">
        <p>Not signed in.</p>
        <button
          className="rounded bg-blue-600 px-4 py-2 text-white"
          onClick={() => window.location.reload()}
        >
          Sign in
        </button>
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
