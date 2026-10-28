import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { demoSong, loadWasm } from "./wasm";
import "./styles.css";

loadWasm().then(() => {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App initial={demoSong()} />
    </StrictMode>,
  );
});
