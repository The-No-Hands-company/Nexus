import { Routes, Route, Navigate } from "react-router-dom";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./store";
import { isTauri, invoke } from "./invoke";
import LoginPage from "./pages/Login";
import RegisterPage from "./pages/Register";
import MainLayout from "./pages/MainLayout";
import OverlayPage from "./pages/Overlay";
import UpdateBanner from "./components/UpdateBanner";
import ThemeProvider from "./themes/ThemeProvider";
import PluginLoader from "./plugins/PluginLoader";
import SearchModal from "./components/SearchModal";
import { initAppearance } from "./themes/appearance";

// Restore font-size and message density classes before first render
initAppearance();

export default function App() {
  const { session, setUpdateAvailable } = useStore();
  const [searchOpen, setSearchOpen] = useState(false);

  // On mount (or hot-reload), re-sync the server URL from the persisted session
  // so the Rust backend and JS _serverUrl stay in sync.
  useEffect(() => {
    if (session?.serverUrl) {
      invoke("set_server_url", { url: session.serverUrl }).catch(() => {});
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Cmd+K / Ctrl+K global search shortcut
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        if (session) setSearchOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [session]);

  // Listen for update-available event from the Tauri updater plugin (Tauri only)
  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen<{ version: string; body: string }>(
      "update-available",
      (e) => {
        setUpdateAvailable(e.payload);
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setUpdateAvailable]);

  // Overlay window gets its own minimal route
  if (window.location.pathname.startsWith("/overlay")) {
    return (
      <ThemeProvider>
        <OverlayPage />
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider>
      {/* Background iframe sandboxes for enabled plugins */}
      <PluginLoader />

      {/* Skip navigation link — visible only when focused (for keyboard users) */}
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-[9999] focus:px-4 focus:py-2 focus:rounded focus:bg-accent-500 focus:text-white focus:text-sm focus:font-medium focus:shadow-lg"
      >
        Skip to content
      </a>

      <div className="flex flex-col h-full">
        <UpdateBanner />
        {searchOpen && session && (
          <SearchModal onClose={() => setSearchOpen(false)} />
        )}
        <main id="main-content" className="flex-1 overflow-hidden">
          <Routes>
            <Route
              path="/login"
              element={session ? <Navigate to="/" replace /> : <LoginPage />}
            />
            <Route
              path="/register"
              element={session ? <Navigate to="/" replace /> : <RegisterPage />}
            />
            <Route
              path="/*"
              element={session ? <MainLayout /> : <Navigate to="/login" replace />}
            />
          </Routes>
        </main>
      </div>
    </ThemeProvider>
  );
}
