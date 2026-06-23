# proute

`proute` discovers Rust page modules and generates route helper modules.

It follows the Elm Land and Gleam `proute` idea that the file path is the route,
then adds a small server-side convention for HTTP mutation endpoints.

## Route Convention

```text
index.rs                       -> GET    /
orders/index.rs                -> GET    /orders
orders/new.rs                  -> GET    /orders/new
orders/create.rs               -> POST   /orders
orders/order_id_/index.rs      -> GET    /orders/{order_id}
orders/order_id_/edit.rs       -> GET    /orders/{order_id}/edit
orders/order_id_/update.rs     -> POST   /orders/{order_id}
orders/order_id_/delete.rs     -> DELETE /orders/{order_id}
```

Rules:

- File paths are routes.
- Trailing underscores mark dynamic path params.
- GET is the default.
- `create.rs`, `update.rs`, and `delete.rs` are reserved mutation endpoints.
- `index.rs` owns the current directory path. At the mount root, it owns `/`.
- `not_found_.rs` owns the mount 404 route.
- `all_.rs` is reserved for future catch-all routing.
- `mod.rs` and every `shared/` directory are ignored.
- `show.rs` is rejected. Use `orders/order_id_/index.rs`.
- A route file cannot also be a namespace parent. If `orders/` exists, use
  `orders/index.rs` instead of `orders.rs`.
- `create`, `update`, and `delete` cannot be used as intermediate path
  segments. They are action files only.

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

## Handler Convention

Each routable module exposes a `handler` function by default:

```rust
pub(crate) async fn handler(...) -> impl IntoResponse
```

`proute` validates this during discovery. The handler may be `pub(crate)` or
`pub`, and may be `async` or sync.

The mount can override that name:

```rust
Mount::new("public", "src/pages/public", "/", "crate::pages::public")
    .with_handler_name("route")
```

When a mount has a router state type, the generated module includes Axum router
functions:

```rust
Mount::new("admin", "src/pages/admin", "/admin", "crate::pages::admin")
    .with_router_state_type("crate::app::AppState")
```

This emits:

```rust
pub fn routes() -> axum::Router<crate::app::AppState>
pub fn prefixed_routes() -> axum::Router<crate::app::AppState>
```

`prefixed_routes` is generated only for mounts with a language param.

## Parsing

Generated modules expose method-aware parsing:

```rust
pub fn parse_request(method: &str, raw_path: &str) -> Route
```

The method is required because HTTP actions can share a path with GET pages:

```text
GET  /orders -> Route::Orders
POST /orders -> Route::OrdersCreate
```

Dynamic path params are percent-decoded after path segmentation, so encoded
slashes stay inside a param.

Generated path helpers percent-encode dynamic params, so a value like `a/b`
round-trips as `/orders/a%2Fb`.

For mounts with a language param, generated modules also expose:

```rust
pub struct ParsedRequest {
    pub route: Route,
    pub lang: Option<String>,
}

pub fn parse_localized_request(method: &str, raw_path: &str) -> ParsedRequest
```

Localized parsing tries the canonical path first. `/orders` returns
`lang: None`, while `/fr/orders` returns `lang: Some("fr")`.
