import { Routes, Route, Navigate } from "react-router-dom";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./store";
import { isTauri } from "./invoke";
import LoginPage from "./pages/Login";
import RegisterPage from "./pages/Register";
import MainLayout from "./pages/MainLayout";
import OverlayPage from "./pages/Overlay";
import UpdateBanner from "./components/UpdateBanner";
import ThemeProvider from "./themes/ThemeProvider";
import PluginLoader from "./plugins/PluginLoader";
import SearchModal from "./components/SearchModal";

export default function App() {
  const { session, setUpdateAvailable } = useStore();
  const [searchOpen, setSearchOpen] = useState(false);

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

      <div className="flex flex-col h-full">
        <UpdateBanner />
        {searchOpen && session && (
          <SearchModal onClose={() => setSearchOpen(false)} />
        )}
        <div className="flex-1 overflow-hidden">
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
        </div>
      </div>
    </ThemeProvider>
  );
}
