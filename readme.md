# Chords

**shortcuts reimagined.**

> This app isn't ready for general use. It's currently only released as a way for me to dogfood it myself.

**Chords** introduces the _chord_, an alternative to keyboard shortcuts which let you trigger actions by typing plain characters without modifier keys.

## How does it work?

Chords are sequences of letters and/or numbers that correspond to actions. Usually, these actions are simply keyboard shortcuts which already have an associated action in an existing app.

For example, here are some example chords for the macOS Finder app:
```toml
# chords/com/apple/finder/macos.toml
gd = { name = "Go to Downloads", shortcut = "opt+cmd+l" }
fu = { name = "Folder Up", shortcut = "cmd+up" }
tt = { name = "Toggle Tabs", shortcut = "cmd+shift+t" }
ts = { name = "Toggle Sidebar", shortcut = "ctrl+cmd+s" }
tp = { name = "Toggle Preview", shortcut = "cmd+shift+p" }
nds = { name = "New Directory with Selection", shortcut = "ctrl+cmd+n" }
```

These chords will only be enabled when the Finder app (which has the bundle identifier `com.apple.finder`) is focused. In addition to application-specific chords, you're also able to define global chords by starting the key sequence with a non-alphanumeric character:

```toml
# chords/macos.toml
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
  # chords/com/apple/finder/macos.toml
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
## JavaScript Scripting

In addition to running shortcuts and shell commands, chords can also run arbitrary JavaScript scripts, which provides more power for certain use-cases, especially for apps that don't necessarily have shortcuts bound to every action.

```toml
# chords/com/microsoft/VSCode/macos.toml
[config.js]
module = '''
export default (commandId: string) => {
  // ...
}

export const menu = (...segments: string[]) => {
  // ...
}
'''

[chords]
# `explorer.newFile` doesn't have a default shortcut in VSCode
fh = { name = "File: Here", args = ["explorer.newFile"] }
# `menu:args` calls the named `menu` export instead of `default`
mc = { name = "Menu: Columns", 'menu:args' = ["View", "Columns"] }
# ...
```

Chords embeds the QuickJS JavaScript environment (excluding its standard library) as well as certain LLRT modules (which are based on the Node APIs). Module resolution is currently only implemented for root imports (e.g. if you have a `src/file.js` at the root of your repo, you have to write `import file from "src/file.js"`).

## Global Hotkeys

Many macOS apps can only be activated through a global hotkey. We thus use a synthetic hotkey pool:
- `cmd+ctrl+f{13..20}`
- `cmd+ctrl+2`
- `cmd+ctrl+3`
- `cmd+ctrl+4`
- `cmd+ctrl+5`
- `cmd+ctrl+6`
- `cmd+ctrl+7`
- `cmd+ctrl+8`
- `cmd+ctrl+9`

## Comparison to keyboard shortcuts

Because keyboard shortcuts must be composed of one or more modifier keys followed by a letter/number/symbol, they come with inherent limitations:

### Limited key combinations

Because you can only choose one of 26 letters for your shortcut, many shortcuts end up with letters that don't intuitively map to their action:

```toml
# chords/com/microsoft/VSCode/macos.toml
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
# chords/com/microsoft/VSCode/macos.toml
gd = { name = "Go to definition", shortcut = "f12" }
rs = { name = "Rename Symbol", shortcut = "f2" }
rf = { name = "Recent Files", shortcut = "cmd+e" }
cp = { name = "Command Palette", shortcut = "cmd+shift+p" }
fc = { name = "Format Code", shortcut = "shift+alt+f" }
```

```toml
# chords/com/jetbrains/intellij/macos.toml
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
