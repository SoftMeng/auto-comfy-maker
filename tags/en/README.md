# English tags (en)

This directory stores **English prompt tags**. Each `.txt` file corresponds to a dimension.

## File naming

Use English dimension names as filenames:

- `hairstyle.txt`
- `jewelry.txt`
- `scene.txt`
- `outfit.txt`
- `expression.txt`
- `composition.txt`

## File format

One tag per line:

```
long hair
short hair
curly hair
updo
```

## When loaded

Loaded when `--lang en` is specified. Useful for Stable Diffusion models
that respond better to English keywords (e.g., SDXL, Flux).

## Adding a new dimension

1. Create `<dimension>.txt` in this directory.
2. Add the dimension name to `[prompt].default_dimensions` in `config/default.toml`.
3. No Rust code changes needed (auto-discovered).

## Adding a new tag

CLI:
```bash
cargo run -- tags add hairstyle "long hair"
```
Or edit the file directly and commit.