import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./global.css";
import { ChordIndicatorWindow } from "./components/indicator";
import { SettingsWindow } from "./windows/settings";

function App() {
  useEffect(() => {
    const rootElement = document.getElementById("root");
    const fullscreenClasses = ["m-0", "h-full", "w-full", "bg-transparent", "p-0"];

    [document.documentElement, document.body, rootElement].forEach((element) => {
      element?.classList.add(...fullscreenClasses);
    });

    return () => {
      [document.documentElement, document.body, rootElement].forEach((element) => {
        element?.classList.remove(...fullscreenClasses);
      });
    };
  }, []);

  const windowLabel = getCurrentWindow().label;

  if (windowLabel === "indicator") {
    return <ChordIndicatorWindow />;
  }

  return <SettingsWindow />;
}

export default App;
