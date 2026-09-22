import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
// Self-hosted so the window renders identically offline; the design's typeface.
import "@fontsource-variable/instrument-sans";
import "./styles/tokens.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
