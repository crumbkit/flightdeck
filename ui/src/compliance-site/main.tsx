import "./site.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { inlineSource } from "./inlineSource.ts";
import { Site } from "./Site.tsx";

const element = document.getElementById("root");
if (element === null) {
  throw new Error("compliance.html has no #root element");
}

createRoot(element).render(
  <StrictMode>
    <Site source={inlineSource(document)} />
  </StrictMode>,
);
