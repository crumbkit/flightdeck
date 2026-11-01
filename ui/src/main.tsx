import "./index.css";
import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/App";
import { applyTheme } from "@/components/ThemeControl";
import { takeLaunchCode } from "@/launch";
import { makeQueryClient } from "@/queryClient";
import { readTheme } from "@/storage";

// The launch code leaves the address bar before React renders anything.
const code = takeLaunchCode(window.location, window.history);
applyTheme(readTheme());

const element = document.getElementById("root");
if (element === null) {
  throw new Error("index.html has no #root element");
}

createRoot(element).render(
  <StrictMode>
    <QueryClientProvider client={makeQueryClient()}>
      <App code={code} />
    </QueryClientProvider>
  </StrictMode>,
);
