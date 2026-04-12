import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./index.css";
import "./i18n";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <React.Suspense fallback={<div className="min-h-screen flex items-center justify-center">Loading translations…</div>}>
        <App />
      </React.Suspense>
    </BrowserRouter>
  </React.StrictMode>
);
