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
orders/order_id_/delete.rs     -> POST   /orders/{order_id}/delete
```

Rules:

- File paths are routes.
- Trailing underscores mark dynamic path params. The segment name before the
  underscore becomes the param name.
- Dynamic segment names do not imply types. `id_`, `slug_`, `product_type_`,
  and `line_item_id_` are names only.
- GET is the default.
- `create.rs`, `update.rs`, and `delete.rs` are reserved mutation endpoints.
- `index.rs` owns the current directory path. At the mount root, it owns `/`.
- `not_found_.rs` is optional. When present, it owns the mount 404 route.
- `all_.rs` is reserved for future catch-all routing.
- `mod.rs` and every `shared/` directory are ignored.
- `show.rs` is rejected. Use `orders/order_id_/index.rs`.
- A route file cannot also be a namespace parent. If `orders/` exists, use
  `orders/index.rs` instead of `orders.rs`.
- `create`, `update`, and `delete` cannot be used as intermediate path
  segments. They are action files only.
- Two dynamic child segments at the same directory level are rejected because
  both would match the same path shape. Static siblings are checked first and
  may live beside one dynamic fallback.

## Generated Layout

The generated app code lives under `proute`:

```text
src/generated/proute/mod.rs
src/generated/proute/public.rs
src/generated/proute/admin.rs
```

That keeps imports focused on the app concept:

```rust
crate::generated::proute::public
crate::generated::proute::admin
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

## URL Helpers

Generated helper names preserve dynamic trailing underscores:

```rust
routes::public::orders_order_id_(123)
// /orders/123

routes::public::orders_order_id__line_items_line_item_id_(123, 456)
// /orders/123/line_items/456
```

Untyped helpers accept `impl std::fmt::Display` as a URL-generation
convenience. That does not give `id_` or any other segment a semantic type; it
only serializes a value into a path segment before percent-encoding it.

With a language param, proute also generates:

```rust
routes::public::prefixed_orders_order_id_("fr", 123)
// /fr/orders/123

routes::public::localized_orders_order_id_("en", "en", 123)
// /orders/123

routes::public::localized_orders_order_id_("fr", "en", 123)
// /fr/orders/123
```

## Typed Route Contracts

Parameterized page modules may define a page-local `RouteParams` contract:

```rust
#[derive(proute::serde::Deserialize)]
pub(crate) struct RouteParams {
    pub(crate) order_id: i64,
}

pub(crate) async fn index(
    proute::Path(params): proute::Path<RouteParams>,
) -> impl axum::response::IntoResponse {
    // params.order_id is already typed here.
}
```

When `RouteParams` exists, proute validates that its fields exactly match the
dynamic path params for the route. The struct and fields must be `pub(crate)`
or `pub` so generated helpers can use the same contract:

```rust
routes::public::orders_order_id_(&RouteParams { order_id: 123 })
```

`proute::Path<T>` delegates to Axum's typed path deserializer and turns any path
deserialization failure into `404 Not Found`. A bad typed route param means the
URL does not satisfy the route contract, so handlers do not need local parsing
branches for route shape errors.

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

Apps that prefer action-named handlers can opt in to deriving the handler name
from the route file:

```rust
Mount::new("admin", "src/pages/admin", "/admin", "crate::pages::admin")
    .with_route_action_handler_names()
```

In that mode, `orders/index.rs` expects `index`, `orders/create.rs` expects
`create`, `orders/order_id_/edit.rs` expects `edit`, and `not_found_.rs`
expects `not_found`.

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

Generated path helpers percent-encode dynamic params, so a value like `a/b`
is generated as `/orders/a%2Fb`.
