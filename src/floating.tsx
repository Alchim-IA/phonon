import React from "react";
import ReactDOM from "react-dom/client";
import FloatingWindow from "./components/FloatingWindow";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <FloatingWindow />
  </React.StrictMode>,
);
