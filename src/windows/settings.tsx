import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import {
  checkAccessibilityPermission,
  checkInputMonitoringPermission,
  requestAccessibilityPermission,
  requestInputMonitoringPermission,
} from "tauri-plugin-macos-permissions-api";
import { Button } from "#/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Checkbox } from "#/components/ui/checkbox.tsx";
import { Label } from "#/components/ui/label.tsx";

export function SettingsWindow() {
  const currentWindow = getCurrentWindow();
  const isMacOS = navigator.userAgent.includes("Mac");
  const [accessibilityBusy, setAccessibilityBusy] = useState(false);
  const [inputMonitoringBusy, setInputMonitoringBusy] = useState(false);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [hasAccessibilityPermission, setHasAccessibilityPermission] = useState(!isMacOS);
  const [hasInputMonitoringPermission, setHasInputMonitoringPermission] = useState(!isMacOS);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [autostartStatus, setAutostartStatus] = useState("Checking launch on login...");

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void currentWindow.onCloseRequested((event) => {
      event.preventDefault();
      void currentWindow.hide();
    }).then((callback) => {
      unlisten = callback;
    });

    return () => {
      unlisten?.();
    };
  }, [currentWindow]);

  async function refreshAccessibilityPermissionState() {
    if (!isMacOS) {
      setHasAccessibilityPermission(true);
      return true;
    }

    const granted = await checkAccessibilityPermission();
    setHasAccessibilityPermission(granted);
    return granted;
  }

  async function refreshInputMonitoringPermissionState() {
    if (!isMacOS) {
      setHasInputMonitoringPermission(true);
      return true;
    }

    const granted = await checkInputMonitoringPermission();
    setHasInputMonitoringPermission(granted);
    return granted;
  }

  async function refreshAutostartState() {
    const enabled = await isAutostartEnabled();
    setAutostartEnabled(enabled);
    setAutostartStatus(
      enabled ? "Chords launches automatically when you log in." : "Chords will not launch on login.",
    );
    return enabled;
  }

  async function ensureAccessibilityPermission() {
    const granted = await refreshAccessibilityPermissionState();

    if (granted) {
      return true;
    }

    setAccessibilityBusy(true);
    let updated = false;

    try {
      await requestAccessibilityPermission();
    } finally {
      updated = await refreshAccessibilityPermissionState();
      setAccessibilityBusy(false);
    }

    return updated;
  }

  async function ensureInputMonitoringPermission() {
    const granted = await refreshInputMonitoringPermissionState();

    if (granted) {
      return true;
    }

    setInputMonitoringBusy(true);
    let updated = false;

    try {
      await requestInputMonitoringPermission();
    } finally {
      updated = await refreshInputMonitoringPermissionState();
      setInputMonitoringBusy(false);
    }

    return updated;
  }

  async function handleAutostartChange(nextValue: boolean) {
    setAutostartBusy(true);

    try {
      if (nextValue) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }

      setAutostartEnabled(nextValue);
      setAutostartStatus(
        nextValue ? "Chords launches automatically when you log in." : "Chords will not launch on login.",
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setAutostartStatus(`Launch on login update failed: ${message}`);
      await refreshAutostartState();
    } finally {
      setAutostartBusy(false);
    }
  }

  async function handleAccessibilityButtonClick() {
    if (hasAccessibilityPermission) {
      await invoke("open_accessibility_settings");
      return;
    }

    await ensureAccessibilityPermission();
  }

  async function handleInputMonitoringButtonClick() {
    if (hasInputMonitoringPermission) {
      await invoke("open_input_monitoring_settings");
      return;
    }

    await ensureInputMonitoringPermission();
  }

  useEffect(() => {
    let cancelled = false;

    async function configureWindow() {
      try {
        await refreshAccessibilityPermissionState();
        await refreshInputMonitoringPermissionState();
        await refreshAutostartState();
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : String(error);
        }
      }
    }

    void configureWindow();

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="min-h-full bg-muted/30 px-5 py-4 text-sm text-foreground">
      <div className="mx-auto flex max-w-[420px] flex-col gap-4">
        <div>
          <h1 className="text-[20px] font-semibold">Chords</h1>
          <p className="mt-1 text-muted-foreground">
            Configure the tray app, and the global shortcut.
          </p>
        </div>

        <Card size="sm">
          <CardHeader>
            <CardTitle>Accessibility</CardTitle>
            <CardDescription>
              {hasAccessibilityPermission
                ? "Accessibility permission is enabled."
                : "Accessibility permission is required for automated clicking."}
            </CardDescription>
          </CardHeader>
          <CardContent className="pt-0">
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                void handleAccessibilityButtonClick();
              }}
              disabled={accessibilityBusy}
            >
              {accessibilityBusy
                ? "Requesting..."
                : hasAccessibilityPermission
                  ? "Open Accessibility Settings"
                  : "Grant Accessibility"}
            </Button>
          </CardContent>
        </Card>

        <Card size="sm">
          <CardHeader>
            <CardTitle>Input Monitoring</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 pt-0">
            <p className="text-sm text-muted-foreground">
              {hasInputMonitoringPermission
                ? "Input Monitoring is enabled."
                : "Input Monitoring is required for the Rust-managed global shortcut."}
            </p>
            <p className="text-sm text-muted-foreground">
              macOS applies this permission after Chords restarts.
            </p>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                void handleInputMonitoringButtonClick();
              }}
              disabled={inputMonitoringBusy}
            >
              {inputMonitoringBusy
                ? "Opening..."
                : hasInputMonitoringPermission
                  ? "Open Input Monitoring Settings"
                  : "Grant Input Monitoring"}
            </Button>
          </CardContent>
        </Card>

        <Card size="sm">
          <CardHeader>
            <CardTitle>Launch on Login</CardTitle>
            <CardDescription>{autostartStatus}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 pt-0">
            <div className="flex items-start gap-3">
              <Checkbox
                id="launch-on-login"
                checked={autostartEnabled}
                disabled={autostartBusy}
                onCheckedChange={(checked) => {
                  void handleAutostartChange(checked === true);
                }}
              />
              <div className="space-y-1">
                <Label htmlFor="launch-on-login">Launch Chords on login</Label>
                <p className="text-sm text-muted-foreground">
                  The app stays in the tray, reuses a single instance, and launches hidden on login.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
