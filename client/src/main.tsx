import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import "./index.css";
import { App } from "./App";
import { applyTheme } from "./store/theme";

// Apply persisted theme before first paint to avoid a flash of the default palette.
try {
  const saved = localStorage.getItem("helm-theme");
  if (saved) {
    const colors = JSON.parse(saved)?.state?.colors;
    if (colors) applyTheme(colors);
  }
} catch {
  // malformed storage; fall back to @theme defaults
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </StrictMode>,
);
