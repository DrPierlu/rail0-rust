# Code Generation

This folder contains the generation pipeline for the RAIL0 Rust SDK. The source of truth for the API surface is `rail0-api/doc/openapi.json` (sibling repo). Running the pipeline regenerates the Rust types in `src/types_gen.rs`.

## Run

```bash
cargo run --bin generate
```

## Schema source (in priority order)

1. `RAIL0_SCHEMA_PATH` env var — absolute path to a local `openapi.json`
2. Default: `../rail0-api/doc/openapi.json` (sibling repo on the local filesystem)

## Workflow when the API changes

1. Update `rail0-api/doc/openapi.json` in the sibling repo (or set `RAIL0_SCHEMA_PATH`).
2. Run `cargo run --bin generate` — rewrites `src/types_gen.rs`.
3. Run `cargo build` — the compiler reports every broken reference across the SDK.
4. Fix the type aliases in `src/types.rs` if any schema names changed.
5. Fix method signatures in `src/payments.rs`, `src/tokens.rs`, `src/utils.rs` if shapes changed.

Steps 4 and 5 are guided by the compiler: no manual diffing of the spec is needed.

## Files

| File | Purpose |
|------|---------|
| `generate.rs` | Generation pipeline — run with `cargo run --bin generate` |

## How `generate.rs` works

The generator is a `[[bin]]` target in the main `Cargo.toml`. It reads `rail0-api/doc/openapi.json` (or the path from `RAIL0_SCHEMA_PATH`), maps
OpenAPI object schemas to Rust structs with `#[derive(Debug, Clone, Serialize, Deserialize)]`, and
writes the result to `src/types_gen.rs`.

Primitive schemas (`Address`, `Bytes32`, `Uint256String`) are skipped by the generator — they are
hand-written type aliases in `src/types.rs` because Rust type aliases carry no runtime overhead.

### Adding a generation step

Add a new function to `gen/generate.rs` and call it from `main`:

```rust
fn generate_docs(api: &OpenApi) {
    // read schemas, emit docs, etc.
}

// in main():
generate_docs(&api);
```
