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

- `/f⇪` (`/`, `f`, `Caps Lock`)

You can run a chord multiple times by pressing `Caps Lock` again. Pressing the following sequence of keys in _Chord Mode_ goes up three folders in Finder:

- `fu⇪⇪⇪` (`f`, `u`, `Caps Lock`)

Another way to run a chord is by holding `Shift` while typing the last key of the chord:

- `/F` (`/`, `Shift`, `F`)

This second method is useful for running multiple chords that share a similar prefix. Unlike Caps Lock, it keeps the chord's starting letters in the input buffer so you can type out similar chords just by typing their endings. Pressing these keys in _Chord Mode_ toggles the tabs in Finder, the sidebar, and the preview:

- `tTSP` (`t`, `Shift`, `t`, `s`, `p`)

Which requires a lot less keys than typing out:

- `tTtStP` (or worse, `tt⇪ts⇪tp⇪`)

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

`/FgD⌘ands⇪`

> **Chords** ignores all inputs whenever a modifier key (other than Shift) is held down.

To exit _Chord Mode_, all you need to do is simply release your `Space` key. It's that simple!
