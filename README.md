# proute

`proute` discovers Rust page modules and generates route helper modules.

It follows the Elm Land and Gleam `proute` idea that the file path is the route,
then adds a small server-side convention for HTTP mutation endpoints.

## Route Convention

```text
orders.rs                  -> GET    /orders
orders/new.rs              -> GET    /orders/new
orders/create.rs           -> POST   /orders
orders/order_id_.rs        -> GET    /orders/{order_id}
orders/order_id_/edit.rs   -> GET    /orders/{order_id}/edit
orders/order_id_/update.rs -> POST   /orders/{order_id}
orders/order_id_/delete.rs -> DELETE /orders/{order_id}
```

Rules:

- File paths are routes.
- Trailing underscores mark dynamic path params.
- GET is the default.
- `create.rs`, `update.rs`, and `delete.rs` are reserved mutation endpoints.
- `home_.rs` owns the mount root.
- `not_found_.rs` owns the mount 404 route.
- `all_.rs` is reserved for future catch-all routing.
- `mod.rs` and every `shared/` directory are ignored.

## Generated Layout

The tool name is `proute`, but generated app code lives under `routes`:

```text
src/generated/routes/mod.rs
src/generated/routes/public.rs
src/generated/routes/admin.rs
```

That keeps imports focused on the app concept:

```rust
crate::generated::routes::public
crate::generated::routes::admin
```

## Build Script Usage

```rust
use std::path::Path;

use proute::{Mount, write_mount_files};

fn main() {
    write_mount_files(
        Path::new("src/generated"),
        [
            Mount::new("public", "src/pages/public", "/", "crate::pages::public")
                .with_language_param("lang"),
            Mount::new("admin", "src/pages/admin", "/admin", "crate::pages::admin")
                .with_language_param("lang"),
        ],
    )
    .expect("generate routes");
}
```

Canonical paths are prefix-free. When a mount has a language param, generated
helpers also expose language-prefixed paths and localized helpers that omit the
prefix for the primary language.
