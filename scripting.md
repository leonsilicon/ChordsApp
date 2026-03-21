# JavaScript

## Recommended Setup

Use [pnpm] because it supports specifying a dependency which is subdirectory of a GitHub repository, which is necessary since `LLRT` doesn't have an npm package for its types:

```jsonc
// package.json
{
  "devDependencies": {
    "llrt-types": "github:awslabs/llrt#path:/types",
    // ...
  }
}
```

Make sure your `tsconfig.json` has the `types` property set to `llrt-types`:
```jsonc
// tsconfig.json
{
  "compilerOptions": {
    "types": ["llrt-types"],
    // ...
  }
}
```

## Recommended Packages

- [nano-spawn-compat](https://github.com/leonsilicon/nano-spawn-compat) - A more ergonomic `child_process.spawn`
- [bplist-lossless](https://github.com/leonsilicon/bplist-lossless) - A binary plist parser specifically tailored for edits by avoiding loss of precision during parsing and re-serialization.
- [keycode-ts2](https://github.com/leonsilicon/keycode-ts2) - A TypeScript port of the [Rust `keycode` crate](https://crates.io/crates/keycode) which uses the Chromium keycode names as the source of truth (_Chords_ uses these keycode names as the source of truth).
