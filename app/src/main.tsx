import "@fontsource-variable/inter";
import "@fontsource/barlow-condensed/600.css";
import "@fontsource/barlow-condensed/700.css";
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
import "./styles/stage.css";

applySettings(getSettings());
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
