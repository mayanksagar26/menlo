import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
// Self-hosted so the window renders identically offline; the design's typeface.
import "@fontsource-variable/instrument-sans";
import "./styles/tokens.css";

// Only the real window is transparent; a browser tab keeps the desk colour.
if ("__TAURI_INTERNALS__" in window) document.documentElement.dataset.transparent = "";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
