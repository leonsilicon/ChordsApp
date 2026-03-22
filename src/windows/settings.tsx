import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, ChevronRight, ExternalLink } from "lucide-react";
import { toast } from "sonner";
import {
  checkAccessibilityPermission,
  checkInputMonitoringPermission,
  requestAccessibilityPermission,
  requestInputMonitoringPermission,
} from "tauri-plugin-macos-permissions-api";
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
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#/components/ui/collapsible.tsx";
import { Input } from "#/components/ui/input.tsx";
import { Label } from "#/components/ui/label.tsx";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "#/components/ui/tabs.tsx";

type GitRepoInfo = {
  owner: string;
  name: string;
  slug: string;
  url: string;
  localPath: string;
  headShortSha: string | null;
};

type LocalChordFolderInfo = {
  name: string;
  localPath: string;
};

type ActiveChordInfo = {
  scope: string;
  scopeKind: "global" | "app";
  sequence: string;
  name: string;
  action: string;
};

type ChordGroup = {
  key: string;
  scope: string;
  scopeKind: "global" | "app";
  chords: ActiveChordInfo[];
};

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

async function validateLocalChordFolder(path: string) {
  const fsApi = window.__TAURI__?.fs;
  if (!fsApi) {
    throw new Error("Filesystem plugin is not available.");
  }

  const exists = await fsApi.exists(path);
  if (!exists) {
    throw new Error("Selected folder is no longer available.");
  }

  const entries = await fsApi.readDir(path);
  const hasChordsDirectory = entries.some((entry) => entry.isDirectory && entry.name === "chords");
  if (!hasChordsDirectory) {
    throw new Error("Selected folder must contain a top-level chords directory.");
  }
}

function compareChordGroups(left: ChordGroup, right: ChordGroup) {
  if (left.scopeKind !== right.scopeKind) {
    return left.scopeKind === "global" ? -1 : 1;
  }

  return left.scope.localeCompare(right.scope);
}

function buildChordGroups(chords: ActiveChordInfo[]): ChordGroup[] {
  const chordGroups: ChordGroup[] = [];
  const chordGroupMap = new Map<string, ChordGroup>();

  for (const chord of chords) {
    const key = `${chord.scopeKind}:${chord.scope}`;
    let group = chordGroupMap.get(key);
    if (!group) {
      group = { key, scope: chord.scope, scopeKind: chord.scopeKind, chords: [] };
      chordGroupMap.set(key, group);
      chordGroups.push(group);
    }

    group.chords.push(chord);
  }

  chordGroups.sort(compareChordGroups);

  for (const group of chordGroups) {
    group.chords.sort(
      (left, right) =>
        left.sequence.localeCompare(right.sequence)
        || left.name.localeCompare(right.name)
        || left.action.localeCompare(right.action),
    );
  }

  return chordGroups;
}

function ChordGroupList({
  groups,
  forceOpen = false,
  openGroups,
  onGroupOpenChange,
}: {
  groups: ChordGroup[];
  forceOpen?: boolean;
  openGroups: Record<string, boolean>;
  onGroupOpenChange: (groupKey: string, open: boolean) => void;
}) {
  return (
    <div className="space-y-2">
      {groups.map((group) => {
        const isOpen = forceOpen || openGroups[group.key] === true;

        return (
          <Collapsible
            key={group.key}
            open={isOpen}
            onOpenChange={(open) => {
              onGroupOpenChange(group.key, open);
            }}
          >
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="flex w-full items-center gap-2 rounded-md border bg-background/80 px-2.5 py-1.5 text-left hover:bg-muted/70"
              >
                <ChevronRight
                  className={`size-3.5 shrink-0 transition-transform ${isOpen ? "rotate-90" : ""}`}
                />
                <Badge variant={group.scopeKind === "global" ? "secondary" : "outline"}>
                  {group.scopeKind === "global" ? "Global" : "App"}
                </Badge>
                <span className="min-w-0 flex-1 truncate text-sm font-medium">{group.scope}</span>
                <span className="text-xs text-muted-foreground">{group.chords.length}</span>
              </button>
            </CollapsibleTrigger>

            <CollapsibleContent className="pt-1">
              <div className="overflow-hidden rounded-md border bg-background/80">
                {group.chords.map((chord) => (
                  <div
                    key={`${chord.scopeKind}:${chord.scope}:${chord.sequence}:${chord.name}`}
                    className="grid grid-cols-[86px_minmax(0,1fr)] gap-x-3 border-b px-2.5 py-1.5 text-xs last:border-b-0"
                  >
                    <div className="truncate font-mono text-[11px] text-foreground/85">
                      {chord.sequence}
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-baseline gap-2">
                        <span className="truncate font-medium">{chord.name}</span>
                        <span className="truncate text-muted-foreground">{chord.action}</span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </CollapsibleContent>
          </Collapsible>
        );
      })}
    </div>
  );
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
  const [addingRepo, setAddingRepo] = useState(false);
  const [syncingRepo, setSyncingRepo] = useState<string | null>(null);
  const [localChordFolders, setLocalChordFolders] = useState<LocalChordFolderInfo[]>([]);
  const [localChordFoldersBusy, setLocalChordFoldersBusy] = useState(true);
  const [addingLocalChordFolder, setAddingLocalChordFolder] = useState(false);
  const [activeChords, setActiveChords] = useState<ActiveChordInfo[]>([]);
  const [activeChordsBusy, setActiveChordsBusy] = useState(true);
  const [chordSearch, setChordSearch] = useState("");
  const [openChordGroups, setOpenChordGroups] = useState<Record<string, boolean>>({});
  const [repoChordsByRepo, setRepoChordsByRepo] = useState<Record<string, ActiveChordInfo[]>>({});
  const [repoChordsBusy, setRepoChordsBusy] = useState<Record<string, boolean>>({});
  const [openRepoChords, setOpenRepoChords] = useState<Record<string, boolean>>({});
  const [openRepoChordGroups, setOpenRepoChordGroups] = useState<
    Record<string, Record<string, boolean>>
  >({});
  const [localFolderChordsByPath, setLocalFolderChordsByPath] = useState<
    Record<string, ActiveChordInfo[]>
  >({});
  const [localFolderChordsBusy, setLocalFolderChordsBusy] = useState<Record<string, boolean>>({});
  const [openLocalFolderChords, setOpenLocalFolderChords] = useState<Record<string, boolean>>({});
  const [openLocalFolderChordGroups, setOpenLocalFolderChordGroups] = useState<
    Record<string, Record<string, boolean>>
  >({});

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

  async function refreshRepos(options?: { showSuccessToast?: boolean; showErrorToast?: boolean }) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setReposBusy(true);

    try {
      const nextRepos = await invoke<GitRepoInfo[]>("list_git_repos");
      setRepos(nextRepos);

      if (showSuccessToast) {
        toast.success("Repo list refreshed.");
      }

      return nextRepos;
    } catch (error) {
      const message = `Failed to load repos: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setReposBusy(false);
    }
  }

  async function refreshLocalChordFolders(options?: {
    showSuccessToast?: boolean;
    showErrorToast?: boolean;
  }) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setLocalChordFoldersBusy(true);

    try {
      const nextFolders = await invoke<LocalChordFolderInfo[]>("list_local_chord_folders_command");
      setLocalChordFolders(nextFolders);

      if (showSuccessToast) {
        toast.success("Local folder list refreshed.");
      }

      return nextFolders;
    } catch (error) {
      const message = `Failed to load local folders: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setLocalChordFoldersBusy(false);
    }
  }

  async function refreshActiveChords(options?: {
    showSuccessToast?: boolean;
    showErrorToast?: boolean;
  }) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setActiveChordsBusy(true);

    try {
      const nextChords = await invoke<ActiveChordInfo[]>("list_active_chords_command");
      setActiveChords(nextChords);

      if (showSuccessToast) {
        toast.success("Active chord list refreshed.");
      }

      return nextChords;
    } catch (error) {
      const message = `Failed to load active chords: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setActiveChordsBusy(false);
    }
  }

  async function refreshRepoChords(
    repoSlug: string,
    options?: { showSuccessToast?: boolean; showErrorToast?: boolean },
  ) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setRepoChordsBusy((current) => ({ ...current, [repoSlug]: true }));

    try {
      const nextChords = await invoke<ActiveChordInfo[]>("list_repo_chords_command", { repo: repoSlug });
      setRepoChordsByRepo((current) => ({ ...current, [repoSlug]: nextChords }));
      setOpenRepoChordGroups((current) => {
        const next = { ...(current[repoSlug] ?? {}) };

        for (const chord of nextChords) {
          const groupKey = `${chord.scopeKind}:${chord.scope}`;
          if (next[groupKey] === undefined) {
            next[groupKey] = chord.scopeKind === "global";
          }
        }

        return { ...current, [repoSlug]: next };
      });

      if (showSuccessToast) {
        toast.success(`Loaded chords from ${repoSlug}.`);
      }

      return nextChords;
    } catch (error) {
      const message = `Failed to load chords from ${repoSlug}: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setRepoChordsBusy((current) => ({ ...current, [repoSlug]: false }));
    }
  }

  async function refreshLocalFolderChords(
    folderPath: string,
    options?: { showSuccessToast?: boolean; showErrorToast?: boolean },
  ) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setLocalFolderChordsBusy((current) => ({ ...current, [folderPath]: true }));

    try {
      const nextChords = await invoke<ActiveChordInfo[]>("list_local_chord_folder_chords_command", {
        path: folderPath,
      });
      setLocalFolderChordsByPath((current) => ({ ...current, [folderPath]: nextChords }));
      setOpenLocalFolderChordGroups((current) => {
        const next = { ...(current[folderPath] ?? {}) };

        for (const chord of nextChords) {
          const groupKey = `${chord.scopeKind}:${chord.scope}`;
          if (next[groupKey] === undefined) {
            next[groupKey] = chord.scopeKind === "global";
          }
        }

        return { ...current, [folderPath]: next };
      });

      if (showSuccessToast) {
        toast.success("Loaded chords from local folder.");
      }

      return nextChords;
    } catch (error) {
      const message = `Failed to load local folder chords: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setLocalFolderChordsBusy((current) => ({ ...current, [folderPath]: false }));
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
      toast.success(nextValue ? "Launch on login enabled." : "Launch on login disabled.");
    } catch (error) {
      const message = getErrorMessage(error);
      setAutostartStatus(`Launch on login update failed: ${message}`);
      toast.error(`Launch on login update failed: ${message}`);
      await refreshAutostartState();
    } finally {
      setAutostartBusy(false);
    }
  }

  async function handleAccessibilityButtonClick() {
    try {
      if (hasAccessibilityPermission) {
        await invoke("open_accessibility_settings");
        toast.info("Opened Accessibility settings.");
        return;
      }

      const granted = await ensureAccessibilityPermission();
      if (granted) {
        toast.success("Accessibility permission granted.");
      } else {
        toast.error("Accessibility permission was not granted.");
      }
    } catch (error) {
      toast.error(`Accessibility action failed: ${getErrorMessage(error)}`);
    }
  }

  async function handleInputMonitoringButtonClick() {
    try {
      if (hasInputMonitoringPermission) {
        await invoke("open_input_monitoring_settings");
        toast.info("Opened Input Monitoring settings.");
        return;
      }

      const granted = await ensureInputMonitoringPermission();
      if (granted) {
        toast.success("Input Monitoring permission granted.");
      } else {
        toast.error("Input Monitoring permission was not granted.");
      }
    } catch (error) {
      toast.error(`Input Monitoring action failed: ${getErrorMessage(error)}`);
    }
  }

  async function handleAddRepo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!repoInput.trim()) {
      toast.error("Enter a GitHub repo like owner/name or https://github.com/owner/name.");
      return;
    }

    setAddingRepo(true);
    const toastId = toast.loading(`Adding ${repoInput.trim()}...`);

    try {
      const addedRepo = await invoke<GitRepoInfo>("add_git_repo_command", { repo: repoInput });
      setRepoInput("");
      await Promise.all([
        refreshRepos({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
      ]);
      toast.success(`Added ${addedRepo.slug}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to add repo: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setAddingRepo(false);
    }
  }

  async function handleAddLocalChordFolder() {
    let selectedPath: string | null = null;

    try {
      selectedPath = await invoke<string | null>("pick_local_chord_folder_command");
      if (!selectedPath) {
        return;
      }
    } catch (error) {
      toast.error(`Failed to choose folder: ${getErrorMessage(error)}`);
      return;
    }

    setAddingLocalChordFolder(true);
    const toastId = toast.loading("Adding local folder...");

    try {
      await validateLocalChordFolder(selectedPath);
      const addedFolder = await invoke<LocalChordFolderInfo>("add_local_chord_folder_command", {
        path: selectedPath,
      });
      await Promise.all([
        refreshLocalChordFolders({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
      ]);
      toast.success(`Added ${addedFolder.name}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to add local folder: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setAddingLocalChordFolder(false);
    }
  }

  async function handleSyncRepo(repoSlug: string) {
    setSyncingRepo(repoSlug);
    const toastId = toast.loading(`Syncing ${repoSlug}...`);

    try {
      const syncedRepo = await invoke<GitRepoInfo>("sync_git_repo_command", { repo: repoSlug });
      setRepoChordsByRepo((current) => {
        const next = { ...current };
        delete next[repoSlug];
        return next;
      });
      setOpenRepoChordGroups((current) => {
        const next = { ...current };
        delete next[repoSlug];
        return next;
      });
      await Promise.all([
        refreshRepos({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
        openRepoChords[repoSlug]
          ? refreshRepoChords(repoSlug, { showErrorToast: false })
          : Promise.resolve([]),
      ]);
      const revisionLabel = syncedRepo.headShortSha ? ` @ ${syncedRepo.headShortSha}` : "";
      toast.success(`Synced ${syncedRepo.slug}${revisionLabel}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to sync ${repoSlug}: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setSyncingRepo(null);
    }
  }

  async function handleOpenRepoUrl(repo: GitRepoInfo) {
    try {
      await openUrl(repo.url);
      toast.info(`Opened ${repo.slug} on GitHub.`);
    } catch (error) {
      toast.error(`Failed to open ${repo.slug}: ${getErrorMessage(error)}`);
    }
  }

  async function handleRepoChordsToggle(repoSlug: string, nextOpen: boolean) {
    setOpenRepoChords((current) => ({ ...current, [repoSlug]: nextOpen }));

    if (!nextOpen || repoChordsByRepo[repoSlug] || repoChordsBusy[repoSlug]) {
      return;
    }

    await refreshRepoChords(repoSlug);
  }

  async function handleLocalFolderChordsToggle(folderPath: string, nextOpen: boolean) {
    setOpenLocalFolderChords((current) => ({ ...current, [folderPath]: nextOpen }));

    if (!nextOpen || localFolderChordsByPath[folderPath] || localFolderChordsBusy[folderPath]) {
      return;
    }

    await refreshLocalFolderChords(folderPath);
  }

  useEffect(() => {
    let cancelled = false;

    async function configureWindow() {
      try {
        await Promise.all([
          refreshAccessibilityPermissionState(),
          refreshInputMonitoringPermissionState(),
          refreshAutostartState(),
          refreshRepos({ showErrorToast: true }),
          refreshLocalChordFolders({ showErrorToast: true }),
          refreshActiveChords({ showErrorToast: true }),
        ]);
      } catch (error) {
        if (!cancelled) {
          toast.error(`Failed to finish loading settings: ${getErrorMessage(error)}`);
        }
      }
    }

    void configureWindow();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setOpenChordGroups((current) => {
      const next = { ...current };

      for (const chord of activeChords) {
        const groupKey = `${chord.scopeKind}:${chord.scope}`;
        if (next[groupKey] === undefined) {
          next[groupKey] = chord.scopeKind === "global";
        }
      }

      return next;
    });
  }, [activeChords]);

  const normalizedChordSearch = chordSearch.trim().toLowerCase();
  const filteredActiveChords = normalizedChordSearch
    ? activeChords.filter((chord) =>
        [chord.scope, chord.sequence, chord.name, chord.action].some((value) =>
          value.toLowerCase().includes(normalizedChordSearch),
        ),
      )
    : activeChords;
  const chordGroups = buildChordGroups(filteredActiveChords);

  return (
    <div className="min-h-full bg-muted/30 px-5 py-4 text-sm text-foreground">
      <div className="mx-auto flex max-w-[720px] flex-col gap-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h1 className="text-[20px] font-semibold">Chords</h1>
            <p className="mt-1 text-muted-foreground">
              Configure the tray app, manage chord sources, and inspect the active chord registry.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="outline">{repos.length + localChordFolders.length} sources</Badge>
            <Badge variant="outline">{activeChords.length} chords</Badge>
          </div>
        </div>

        <Tabs defaultValue="settings" className="gap-4">
          <TabsList>
            <TabsTrigger value="settings">Settings</TabsTrigger>
            <TabsTrigger value="active-chords">Active Chords</TabsTrigger>
          </TabsList>

          <TabsContent value="settings" className="space-y-4">
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
                      void refreshRepos({ showSuccessToast: true });
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
                        className="rounded-lg border bg-background/80 px-3 py-3"
                      >
                        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                          <div className="min-w-0 space-y-1">
                            <div className="flex items-center gap-2">
                              <p className="truncate font-medium">{repo.slug}</p>
                              <Badge variant="secondary">GitHub</Badge>
                              {repo.headShortSha ? (
                                <Badge variant="outline" className="font-mono text-[11px]">
                                  {repo.headShortSha}
                                </Badge>
                              ) : null}
                            </div>
                          </div>
                          <div className="flex flex-wrap items-center gap-2 self-end sm:self-center">
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon-sm"
                              aria-label={`Open ${repo.slug} on GitHub`}
                              title="Open on GitHub"
                              onClick={() => {
                                void handleOpenRepoUrl(repo);
                              }}
                            >
                              <ExternalLink />
                            </Button>
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              onClick={() => {
                                void handleRepoChordsToggle(repo.slug, !openRepoChords[repo.slug]);
                              }}
                              disabled={repoChordsBusy[repo.slug] === true}
                            >
                              {openRepoChords[repo.slug] ? "Hide Chords" : "View Chords"}
                            </Button>
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
                        </div>

                        <Collapsible open={openRepoChords[repo.slug] === true}>
                          <CollapsibleContent className="pt-3">
                            {repoChordsBusy[repo.slug] === true ? (
                              <p className="text-sm text-muted-foreground">
                                Loading chords from {repo.slug}...
                              </p>
                            ) : (repoChordsByRepo[repo.slug]?.length ?? 0) === 0 ? (
                              <p className="text-sm text-muted-foreground">
                                No chords found in {repo.slug}.
                              </p>
                            ) : (
                              <ChordGroupList
                                groups={buildChordGroups(repoChordsByRepo[repo.slug] ?? [])}
                                openGroups={openRepoChordGroups[repo.slug] ?? {}}
                                onGroupOpenChange={(groupKey, open) => {
                                  setOpenRepoChordGroups((current) => ({
                                    ...current,
                                    [repo.slug]: {
                                      ...(current[repo.slug] ?? {}),
                                      [groupKey]: open,
                                    },
                                  }));
                                }}
                              />
                            )}
                          </CollapsibleContent>
                        </Collapsible>
                      </div>
                    ))
                  )}
                </div>
              </CardContent>
            </Card>

            <Card size="sm">
              <CardHeader>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <CardTitle>Local Folders</CardTitle>
                    <CardDescription>
                      Local folders are loaded in place. Use the tray reload action after editing
                      files to rebuild the JS runtime.
                    </CardDescription>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      void refreshLocalChordFolders({ showSuccessToast: true });
                    }}
                    disabled={localChordFoldersBusy || addingLocalChordFolder}
                  >
                    {localChordFoldersBusy ? "Refreshing..." : "Refresh"}
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="space-y-4 pt-0">
                <div className="flex justify-end">
                  <Button
                    type="button"
                    onClick={() => {
                      void handleAddLocalChordFolder();
                    }}
                    disabled={addingLocalChordFolder}
                  >
                    {addingLocalChordFolder ? "Adding..." : "Add Folder"}
                  </Button>
                </div>

                <div className="space-y-3">
                  {localChordFoldersBusy ? (
                    <p className="text-sm text-muted-foreground">Loading local folders...</p>
                  ) : localChordFolders.length === 0 ? (
                    <p className="text-sm text-muted-foreground">
                      No local folders added yet.
                    </p>
                  ) : (
                    localChordFolders.map((folder) => (
                      <div
                        key={folder.localPath}
                        className="rounded-lg border bg-background/80 px-3 py-3"
                      >
                        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                          <div className="min-w-0 space-y-1">
                            <div className="flex items-center gap-2">
                              <p className="truncate font-medium">{folder.name}</p>
                              <Badge variant="secondary">Local</Badge>
                            </div>
                            <p className="truncate text-xs text-muted-foreground">
                              {folder.localPath}
                            </p>
                          </div>
                          <div className="flex flex-wrap items-center gap-2 self-end sm:self-center">
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              onClick={() => {
                                void handleLocalFolderChordsToggle(
                                  folder.localPath,
                                  !openLocalFolderChords[folder.localPath],
                                );
                              }}
                              disabled={localFolderChordsBusy[folder.localPath] === true}
                            >
                              {openLocalFolderChords[folder.localPath] ? "Hide Chords" : "View Chords"}
                            </Button>
                          </div>
                        </div>

                        <Collapsible open={openLocalFolderChords[folder.localPath] === true}>
                          <CollapsibleContent className="pt-3">
                            {localFolderChordsBusy[folder.localPath] === true ? (
                              <p className="text-sm text-muted-foreground">
                                Loading chords from {folder.name}...
                              </p>
                            ) : (localFolderChordsByPath[folder.localPath]?.length ?? 0) === 0 ? (
                              <p className="text-sm text-muted-foreground">
                                No chords found in {folder.name}.
                              </p>
                            ) : (
                              <ChordGroupList
                                groups={buildChordGroups(localFolderChordsByPath[folder.localPath] ?? [])}
                                openGroups={openLocalFolderChordGroups[folder.localPath] ?? {}}
                                onGroupOpenChange={(groupKey, open) => {
                                  setOpenLocalFolderChordGroups((current) => ({
                                    ...current,
                                    [folder.localPath]: {
                                      ...(current[folder.localPath] ?? {}),
                                      [groupKey]: open,
                                    },
                                  }));
                                }}
                              />
                            )}
                          </CollapsibleContent>
                        </Collapsible>
                      </div>
                    ))
                  )}
                </div>
              </CardContent>
            </Card>

            <Card size="sm">
              <CardHeader>
                <CardTitle>Permissions</CardTitle>
                <CardDescription>
                  Grant macOS access for clicking chords and listening for the global shortcut.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-2 pt-0">
                <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
                  <div className="min-w-0">
                    <p className="truncate font-medium">Accessibility</p>
                    <p className="truncate text-xs text-muted-foreground">
                      Needed for automated clicking.
                    </p>
                  </div>
                  {hasAccessibilityPermission ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      aria-label="Open Accessibility settings"
                      title="Open Accessibility settings"
                      onClick={() => {
                        void handleAccessibilityButtonClick();
                      }}
                      disabled={accessibilityBusy}
                    >
                      <Check className="text-emerald-600" />
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        void handleAccessibilityButtonClick();
                      }}
                      disabled={accessibilityBusy}
                    >
                      {accessibilityBusy ? "Requesting..." : "Grant"}
                    </Button>
                  )}
                </div>

                <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
                  <div className="min-w-0">
                    <p className="truncate font-medium">Input Monitoring</p>
                    <p className="truncate text-xs text-muted-foreground">
                      Needed for the global shortcut; restart after enabling.
                    </p>
                  </div>
                  {hasInputMonitoringPermission ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      aria-label="Open Input Monitoring settings"
                      title="Open Input Monitoring settings"
                      onClick={() => {
                        void handleInputMonitoringButtonClick();
                      }}
                      disabled={inputMonitoringBusy}
                    >
                      <Check className="text-emerald-600" />
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        void handleInputMonitoringButtonClick();
                      }}
                      disabled={inputMonitoringBusy}
                    >
                      {inputMonitoringBusy ? "Opening..." : "Grant"}
                    </Button>
                  )}
                </div>
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
                      The app stays in the tray, reuses a single instance, and launches hidden on
                      login.
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="active-chords">
            <Card size="sm">
              <CardHeader>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <CardTitle>Registered Chords</CardTitle>
                    <CardDescription>
                      Live view of the chord registry loaded in `context.loaded_app_chords`.
                    </CardDescription>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      void refreshActiveChords({ showSuccessToast: true });
                    }}
                    disabled={activeChordsBusy}
                  >
                    {activeChordsBusy ? "Refreshing..." : "Refresh"}
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="space-y-3 pt-0">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
                  <Input
                    value={chordSearch}
                    onChange={(event) => {
                      setChordSearch(event.target.value);
                    }}
                    placeholder="Filter by app, trigger, name, or action"
                  />
                  <Badge variant="outline" className="self-start sm:self-center">
                    {filteredActiveChords.length} matches
                  </Badge>
                </div>

                {activeChordsBusy ? (
                  <p className="text-sm text-muted-foreground">Loading active chords...</p>
                ) : activeChords.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No chords are currently loaded.</p>
                ) : filteredActiveChords.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No chords match that filter.</p>
                ) : (
                  <ChordGroupList
                    groups={chordGroups}
                    forceOpen={normalizedChordSearch.length > 0}
                    openGroups={openChordGroups}
                    onGroupOpenChange={(groupKey, open) => {
                      setOpenChordGroups((current) => ({
                        ...current,
                        [groupKey]: open,
                      }));
                    }}
                  />
                )}
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
