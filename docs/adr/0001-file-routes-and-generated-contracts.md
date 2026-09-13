# 0001: File Routes and Generated Contracts

## Status

Accepted

## Decision

Proute discovers application-owned Rust page modules and generates Axum route
mounts and URL helpers. A file's path defines its route. `index.rs` owns its
directory path; `create.rs`, `update.rs`, and `delete.rs` define POST actions.
Directories ending in an underscore define dynamic path segments. A terminal
`all_.rs` defines a catch-all, and an optional `not_found_.rs` defines the
mount's 404 route. Discovery rejects ambiguous dynamic siblings and route
files that also act as namespace parents.

Mount configuration owns URL prefixes, language prefixes, static URL spellings
that differ from Rust module names, handler naming, Axum state, and narrow
per-route body limits. Applications own their page modules, handler workflows,
authorization, and state. Proute does not decide application behavior.

A page can declare `RouteParams` to bind dynamic segments to typed fields.
Proute validates the field names against the discovered path and generates URL
helpers using those field types. Incoming typed extraction through
`proute::Path<T>` returns 404 when a path value does not satisfy the route
contract. Dynamic URL values are percent-encoded. `FriendlyId<T>` may append a
readable slug while parsing only the leading typed ID.

Generated code lives in an application-owned `generated/proute` namespace.
Discovery, validation, and emission remain one library workflow. Applications
can regenerate their routes from source rather than maintaining a second route
table.

## Consequences

Page layout, route registration, and URL generation stay aligned at build time.
Changing a page path or its typed parameters requires regenerating route code.
Applications retain control of request handling and authorization.
