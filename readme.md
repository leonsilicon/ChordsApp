# Chords

**shortcuts reimagined.**

> This app isn't ready for general use. It's currently only released as a way for me to dogfood it myself.

**Chords** introduces the _chord_, an alternative to keyboard shortcuts which let you trigger actions by typing plain letters and numbers without modifier keys.

## How does it work?

Chords are sequences of letters and/or numbers that correspond to actions. Usually, these actions are simply keyboard shortcuts which already have an associated action in an existing app.

For example, here are some example chords for the macOS Finder app:
```toml
# chords/macos/com/apple/finder/chords.toml
gd = { name = "Go to Downloads", shortcut = "opt+cmd+l" }
fu = { name = "Folder Up", shortcut = "cmd+up" }
tt = { name = "Toggle Tabs", shortcut = "cmd+shift+t" }
ts = { name = "Toggle Sidebar", shortcut = "ctrl+cmd+s" }
tp = { name = "Toggle Preview", shortcut = "cmd+shift+p" }
nds = { name = "New Directory with Selection", shortcut = "ctrl+cmd+n" }
```

These chords will only be enabled when the Finder app (which has the bundle identifier `com.apple.finder`) is focused. In addition to application-specific chords, you're also able to define global chords by starting the key sequence with a non-alphanumeric character:

```toml
# chords/macos/chords.toml
"/q" = { name = "Force Quit", command = "Force Quit", shortcut = "cmd+opt+esc" }
```

Global chords will always be enabled regardless of which app is focused.

Actions can also take the form of shell commands, which is useful when certain functionality isn't available via a keyboard shortcut:
```toml
"/f" = { name = "Finder", command = "Finder", shell = "open -a Finder" }
```

To run a chord, you need to have the **Chords** app installed and running in the background. **Chords** won't do anything until you press `Caps Lock + Space`, which activates _Chord Mode_.

<details>
  <summary>Why Caps Lock + Space?</summary>

  An ideal requirement for typing chords is to have all your fingers free to type arbitrary chord sequences while a certain key is held down. One of the only keys that fit this requirement is the `Space` key.

  However, `Space` needs to be pressed as part of a key combination, since pressing it alone will output the actual space ` ` character. The key which makes the most sense as part of this combination is `Caps Lock`, since it's one of the easiest keys to reach yet still remains relatively unused on most layouts.

  Because we only use it as part of a key combination, pressing `Caps Lock` on its own will still toggle on Caps as usual, and this special behavior only applies when `Space` is pressed down while `Caps Lock` is pressed.
</details>

_Chord Mode_ stays active as long as the `Space` key is pressed down. In _Chord Mode_, you can type a sequence of letters and/or numbers that corresponds to a defined chord. One way to run a chord is by typing the sequence followed by the `Caps Lock` key:

<details>
  <summary>/f⇪</summary>

  1. Press(Slash)
  2. Release(Slash)
  3. Press(KeyF)
  4. Release(KeyF)
  5. Press(CapsLock)
  6. Release(CapsLock)

  > In future expansions, we use `Tap` to mean a `Press` followed by a `Release`, so this expansion could've also been written as:

  1. Tap(Slash)
  2. Tap(KeyF)
  3. Tap(CapsLock)
</details>

You can run a chord multiple times by pressing `Caps Lock` again. Pressing the following sequence of keys in _Chord Mode_ goes up three folders in Finder:

<details>
  <summary>/f⇪⇪⇪</summary>

  1. Tap(Slash)
  2. Tap(F)
  3. Tap(CapsLock)
  4. Tap(CapsLock)
  5. Tap(CapsLock)
</details>

Another way to run a chord is by holding `Shift` while typing the last key of the chord:

<details>
  <summary>/F</summary>

  1. Tap(Slash)
  2. Press(Shift)
  3. Tap(F)
  4. Release(Shift)
</details>

This second method is useful for running multiple chords that share a similar prefix. Unlike Caps Lock, it keeps the chord's starting letters in the input buffer so you can type out similar chords just by typing their endings.

As an example, say you wanted to quickly toggle the tabs view, the sidebar view, and the preview in Finder. Instead of typing out the entirety of three separate chords:

<details>
  <summary>tTtStP</summary>

  1. Tap(T)
  2. Press(Shift)
  3. Tap(T)
  4. Release(Shift)
  5. Tap(T)
  6. Press(Shift)
  7. Tap(S)
  8. Release(Shift)
  9. Tap(T)
  10. Press(Shift)
  11. Tap(P)
  12. Release(Shift)
</details>

You can just type:

<details>
  <summary>tTSP</summary>

  1. Tap(T)
  3. Press(Shift)
  4. Tap(T)
  5. Tap(S)
  6. Tap(P)
  4. Release(Shift)
</details>

<details>
  <summary>Why do chords require Caps Lock or Shift?</summary>

  While the extra keypress does make it more verbose, it's necessary for distinguishing between certain chords. For example, let's say we have the following chords defined:
  ```toml
  # chords/macos/com/apple/finder/chords.toml
  nd = { name = "New Directory", shortcut = "cmd+shift+n" }
  nds = { name = "New Directory with Selection", shortcut = "ctrl+cmd+n" }
  ```

  If just typing `nd` triggered the first chord, it'd be impossible to run the second chord `nds`. While a possible solution might be to force all chords to be two characters, this ends up sacrificing the flexibility that make chords an appealing alternative to traditional keyboard shortcuts in the first place.

  You might have also noticed that all chords are a minimum of two keys, and this is an intentional rule or else it'd be ambiguous whether pressing `Shift` means starting a new chord or re-using the existing prefix.
</details>

While it might seem a bit tedious to press `Caps Lock` or `Shift`, it's actually a lot smoother in practice since both `Caps Lock` and `Shift` are relatively easy to reach compared to modifier keys. In addition, because chords don't use modifier keys, you're able to use any existing shortcuts while _Chord Mode_ is active. The following sequence of keys will move all the contents of your Downloads folder into a new folder:

<details>
  <summary>/FgD⌘ands⇪</summary>

  1. Tap(Slash)
  2. Press(Shift)
  3. Tap(F)
  4. Release(Shift)
  5. Tap(G)
  6. Press(Shift)
  7. Tap(D)
  8. Release(Shift)
  9. Press(Command)
  10. Tap(A)
  11. Release(Command)
  11. Tap(N)
  11. Tap(D)
  11. Tap(S)
  12. Tap(CapsLock)
</details>

> **Chords** ignores all inputs whenever a modifier key (other than Shift) is held down.

To exit _Chord Mode_, all you need to do is simply release your `Space` key. It's that simple!

<!-- TODO: This section should be introduced alongside `shell` -->
### Lua Scripting

In addition to running shortcuts and shell commands, chords can also run arbitrary Lua scripts, which provides more power for certain use-cases, especially for apps that don't necessarily have shortcuts bound to every action.

For example, this `vscode_helpers.lua` script allows us to programatically execute VSCode commands by their ID using the [Cursorless extension](https://marketplace.visualstudio.com/items?itemName=pokey.command-server):

```lua
-- lua/vscode_helpers.lua
local M = {}

local file = require("file")

local function get_uid()
  local h = io.popen("id -u")
  if not h then return nil end

  local uid = h:read("*a")
  h:close()

  if not uid then return nil end
  return uid:gsub("%s+", "")
end

local function json_escape(s)
  return tostring(s)
    :gsub("\\", "\\\\")
    :gsub('"', '\\"')
end

-- This script allows us to execute VSCode commands directly via https://marketplace.visualstudio.com/items?itemName=pokey.command-server if it's active, and otherwise falls back to built-in shortcuts.
function M.create_command()
  local uid = get_uid()
  if not uid then
    return function() return false end
  end

  local tmp = os.getenv("TMPDIR") or "/tmp"
  local dir = tmp .. "/vscode-command-server-" .. uid

  return function(cmd)
    -- ensure server dir exists
    if os.rename(dir, dir) == nil then
      return false
    end

    local request_path = dir .. "/request.json"
    local response_path = dir .. "/response.json"

    local payload = string.format(
      '{"commandId":"%s","args":[]}',
      json_escape(cmd)
    )

    if not file.write(request_path, payload) then
      return false
    end

    -- remove stale response BEFORE triggering
    os.remove(response_path)

    -- trigger VSCode command server
    tap("cmd+shift+f17")

    return true
  end
end

return M
```

```toml
# chords/macos/com/microsoft/VSCode/chords.toml
[config.lua]
init = '''
command = require("vscode_helpers").create_command()
'''

[chords]
fh = { name = "File: Here", lua = "command('explorer.newFile')" } # doesn't have a default shortcut
# ...
```

The Lua environment provided by Chords uses Lua 5.4 and includes the entire standard library. It provides three helper functions for simulating keypresses: `tap`, `press`, and `release` (in the future, it'll be locked down to avoid arbitrary code execution).

This environment is kept intentionally minimal to avoid tying it to any app-specific functionality.

## Comparison to keyboard shortcuts

Because keyboard shortcuts must be composed of one or more modifier keys followed by a letter/number/symbol, they come with inherent limitations:

### Limited key combinations

Because you can only choose one of 26 letters for your shortcut, many shortcuts end up with letters that don't intuitively map to their action:

```toml
# chords/macos/com/microsoft/VSCode/chords.toml
gf = {
  name = "Go to File",
  # cmd+p doesn't make you think of "File" (my best guess is that cmd+f is already taken by Find, and so it's adapted from the shortcut for the similar feature Command Palette which is cmd+shift+p (p for palette)
  # Either way, "gf" for "goto file" is a lot easier to remember
  shortcut = "cmd+p"
}

gd = {
  name = "Go to Definition",
  # Some shortcuts don't even use letters at all...
  shortcut = "F12"
}
```


### Differences between platforms

The same app on different platforms (Windows/Linux/MacOS) often use different shortcuts for the same action (including different modifier keys), which can be a pain to deal with if you need to switch between platforms.

Chords can act as an abstraction over these shortcut differences by letting you map the same chord to different shortcuts on each platform.

<!-- TODO: give example -->

### Differences between apps

Different apps will often have different keybindings for similar actions. While you are able to set custom keymaps in certain apps, they make it harder to follow along with documentation (which often assume the default keymap) and require you to create and maintain your own keybindings files if you ever want to make changes.

With chords, you can define the same chord across multiple apps which map to the corresponding shortcut for that app. This way, you can just remember one chord for an action and it'll work across all your apps without you having to memorize the specific shortcuts for each app:

```toml
# chords/macos/com/microsoft/VSCode/chords.toml
gd = { name = "Go to definition", shortcut = "f12" }
rs = { name = "Rename Symbol", shortcut = "f2" }
rf = { name = "Recent Files", shortcut = "cmd+e" }
cp = { name = "Command Palette", shortcut = "cmd+shift+p" }
fc = { name = "Format Code", shortcut = "shift+alt+f" }
```

```toml
# chords/macos/com/jetbrains/intellij/chords.toml
gd = { name = "Go to definition", shortcut = "cmd+b" }
rs = { name = "Rename Symbol", shortcut = "shift+f6" }
rf = { name = "Recent Files", shortcut = "ctrl+tab" }
cp = { name = "Command Palette", shortcut = "cmd+shift+a" }
fc = { name = "Format Code", shortcut = "cmd+option+l" }
```

### Multi-modifier combinations are difficult to press and remember
```toml
ss = { name = "Sort by Size", shortcut = "cmd+opt+cmd+6" }
```
