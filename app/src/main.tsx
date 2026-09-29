import "@fontsource-variable/inter";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { applySettings, getSettings } from "./settings";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/ui.css";
import "./styles/table.css";
import "./styles/shell.css";
import "./styles/pages.css";

applySettings(getSettings());
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
