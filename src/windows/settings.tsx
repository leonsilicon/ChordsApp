import { useEffect, useState, type FormEvent } from "react";
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
import { Alert, AlertDescription, AlertTitle } from "#/components/ui/alert.tsx";
import { Badge } from "#/components/ui/badge.tsx";
import { Button } from "#/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Checkbox } from "#/components/ui/checkbox.tsx";
import { Input } from "#/components/ui/input.tsx";
import { Label } from "#/components/ui/label.tsx";

type GitRepoInfo = {
  owner: string;
  name: string;
  slug: string;
  url: string;
  localPath: string;
};

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

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
  const [repos, setRepos] = useState<GitRepoInfo[]>([]);
  const [reposBusy, setReposBusy] = useState(true);
  const [repoInput, setRepoInput] = useState("");
  const [repoStatus, setRepoStatus] = useState("");
  const [repoError, setRepoError] = useState("");
  const [addingRepo, setAddingRepo] = useState(false);
  const [syncingRepo, setSyncingRepo] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void currentWindow
      .onCloseRequested((event) => {
        event.preventDefault();
        void currentWindow.hide();
      })
      .then((callback) => {
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

  async function refreshRepos() {
    try {
      const nextRepos = await invoke<GitRepoInfo[]>("list_git_repos");
      setRepos(nextRepos);
      setRepoError("");
      return nextRepos;
    } catch (error) {
      const message = getErrorMessage(error);
      setRepoError(`Failed to load repos: ${message}`);
      return [];
    } finally {
      setReposBusy(false);
    }
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
      const message = getErrorMessage(error);
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

  async function handleAddRepo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!repoInput.trim()) {
      setRepoError("Enter a GitHub repo like owner/name or https://github.com/owner/name.");
      return;
    }

    setAddingRepo(true);
    setRepoError("");
    setRepoStatus("");

    try {
      const addedRepo = await invoke<GitRepoInfo>("add_git_repo_command", { repo: repoInput });
      setRepoInput("");
      setRepoStatus(`Added ${addedRepo.slug}. Its chords are now loaded.`);
      await refreshRepos();
    } catch (error) {
      setRepoError(`Failed to add repo: ${getErrorMessage(error)}`);
    } finally {
      setAddingRepo(false);
    }
  }

  async function handleSyncRepo(repoSlug: string) {
    setSyncingRepo(repoSlug);
    setRepoError("");
    setRepoStatus("");

    try {
      const syncedRepo = await invoke<GitRepoInfo>("sync_git_repo_command", { repo: repoSlug });
      setRepoStatus(`Synced ${syncedRepo.slug} and reloaded chords.`);
      await refreshRepos();
    } catch (error) {
      setRepoError(`Failed to sync ${repoSlug}: ${getErrorMessage(error)}`);
    } finally {
      setSyncingRepo(null);
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function configureWindow() {
      try {
        await Promise.all([
          refreshAccessibilityPermissionState(),
          refreshInputMonitoringPermissionState(),
          refreshAutostartState(),
          refreshRepos(),
        ]);
      } catch (error) {
        if (!cancelled) {
          setRepoError(`Failed to finish loading settings: ${getErrorMessage(error)}`);
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
      <div className="mx-auto flex max-w-[620px] flex-col gap-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h1 className="text-[20px] font-semibold">Chords</h1>
            <p className="mt-1 text-muted-foreground">
              Configure the tray app, global shortcut permissions, and chord repos.
            </p>
          </div>
          <Badge variant="outline">{repos.length} repos</Badge>
        </div>

        <Card size="sm">
          <CardHeader>
            <div className="flex items-center justify-between gap-3">
              <div>
                <CardTitle>Chord Repos</CardTitle>
                <CardDescription>
                  Added GitHub repos are cloned into the app cache and merged with bundled chords.
                </CardDescription>
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  void refreshRepos();
                }}
                disabled={reposBusy || addingRepo || syncingRepo !== null}
              >
                {reposBusy ? "Refreshing..." : "Refresh"}
              </Button>
            </div>
          </CardHeader>
          <CardContent className="space-y-4 pt-0">
            <form className="flex flex-col gap-3 sm:flex-row" onSubmit={handleAddRepo}>
              <Input
                value={repoInput}
                onChange={(event) => {
                  setRepoInput(event.target.value);
                }}
                placeholder="owner/name or https://github.com/owner/name"
                disabled={addingRepo}
              />
              <Button type="submit" disabled={addingRepo}>
                {addingRepo ? "Adding..." : "Add Repo"}
              </Button>
            </form>

            {repoStatus ? (
              <Alert>
                <AlertTitle>Repo update</AlertTitle>
                <AlertDescription>{repoStatus}</AlertDescription>
              </Alert>
            ) : null}

            {repoError ? (
              <Alert variant="destructive">
                <AlertTitle>Repo error</AlertTitle>
                <AlertDescription>{repoError}</AlertDescription>
              </Alert>
            ) : null}

            <div className="space-y-3">
              {reposBusy ? (
                <p className="text-sm text-muted-foreground">Loading cached repos...</p>
              ) : repos.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No external repos added yet. Bundled chords still load by default.
                </p>
              ) : (
                repos.map((repo) => (
                  <div
                    key={repo.slug}
                    className="flex flex-col gap-3 rounded-lg border bg-background/80 px-3 py-3 sm:flex-row sm:items-center sm:justify-between"
                  >
                    <div className="min-w-0 space-y-1">
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium">{repo.slug}</p>
                        <Badge variant="secondary">GitHub</Badge>
                      </div>
                      <p className="truncate text-sm text-muted-foreground">{repo.url}</p>
                      <p className="truncate text-xs text-muted-foreground">{repo.localPath}</p>
                    </div>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        void handleSyncRepo(repo.slug);
                      }}
                      disabled={addingRepo || syncingRepo === repo.slug}
                    >
                      {syncingRepo === repo.slug ? "Syncing..." : "Sync Latest"}
                    </Button>
                  </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>

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
