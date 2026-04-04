import { Routes, Route, Navigate } from "react-router-dom";
import { useStore } from "./store";
import Login from "./pages/Login";
import Register from "./pages/Register";
import MainLayout from "./pages/MainLayout";

export default function App() {
  const { session } = useStore();

  return (
    <Routes>
      <Route
        path="/login"
        element={session ? <Navigate to="/" replace /> : <Login />}
      />
      <Route
        path="/register"
        element={session ? <Navigate to="/" replace /> : <Register />}
      />
      <Route
        path="/*"
        element={session ? <MainLayout /> : <Navigate to="/login" replace />}
      />
    </Routes>
  );
}
