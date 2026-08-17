import React from "react";
import ReactDOM from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import "@radix-ui/themes/styles.css";
import App from "./App";
import { useSystemAppearance } from "./useSystemAppearance";

function Root() {
  return (
    <Theme appearance={useSystemAppearance()} accentColor="jade" grayColor="slate">
      <App />
    </Theme>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
