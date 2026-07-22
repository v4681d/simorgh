import React from "react"
import ReactDOM from "react-dom/client"
import App from "./App"
import Overlay from "./screens/Overlay"
import "./i18n"
import "./index.css"

// The overlay window loads index.html#/overlay (see crates/app/src/windows.rs).
const isOverlay = window.location.hash.startsWith("#/overlay")

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isOverlay ? <Overlay /> : <App />}</React.StrictMode>,
)
