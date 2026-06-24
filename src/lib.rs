use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::future::Future;
use std::path::{Path as StdPath, PathBuf};
use std::str::FromStr;

use quote::ToTokens;

pub use axum;
pub use serde;

pub trait ToParam {
    fn to_param(&self) -> String;
}

pub trait IntoParam<T> {
    fn into_param(self) -> String;
}

impl ToParam for str {
    fn to_param(&self) -> String {
        self.to_string()
    }
}

impl ToParam for String {
    fn to_param(&self) -> String {
        self.clone()
    }
}

impl<T: ToParam + ?Sized> ToParam for &T {
    fn to_param(&self) -> String {
        (*self).to_param()
    }
}

impl<T> IntoParam<T> for T
where
    T: ToParam,
{
    fn into_param(self) -> String {
        self.to_param()
    }
}

impl<T> IntoParam<T> for &T
where
    T: ToParam,
{
    fn into_param(self) -> String {
        self.to_param()
    }
}

impl IntoParam<String> for &str {
    fn into_param(self) -> String {
        self.to_string()
    }
}

macro_rules! impl_to_param_display {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ToParam for $ty {
                fn to_param(&self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}

impl_to_param_display!(
    bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize,
);

pub const DEFAULT_FRIENDLY_SLUG_MAX_CHARS: usize = 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FriendlyId<T> {
    pub id: T,
    pub slug: Option<String>,
    pub slug_max_chars: usize,
}

impl<T> FriendlyId<T> {
    pub fn new(id: T, slug: impl Into<String>) -> Self {
        Self {
            id,
            slug: Some(slug.into()),
            slug_max_chars: DEFAULT_FRIENDLY_SLUG_MAX_CHARS,
        }
    }

    pub fn id_only(id: T) -> Self {
        Self {
            id,
            slug: None,
            slug_max_chars: DEFAULT_FRIENDLY_SLUG_MAX_CHARS,
        }
    }

    pub fn with_slug_limit(mut self, max_chars: usize) -> Self {
        self.slug_max_chars = max_chars;
        self
    }
}

impl<T: fmt::Display> ToParam for FriendlyId<T> {
    fn to_param(&self) -> String {
        let id = self.id.to_string();
        let Some(slug) = self.slug.as_deref() else {
            return id;
        };
        let suffix = friendly_slug(slug, self.slug_max_chars);
        if suffix.is_empty() {
            id
        } else {
            format!("{id}-{suffix}")
        }
    }
}

impl<T> IntoParam<FriendlyId<T>> for T
where
    T: ToParam,
{
    fn into_param(self) -> String {
        self.to_param()
    }
}

impl<'de, T> serde::Deserialize<'de> for FriendlyId<T>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(std::marker::PhantomData<T>);

        impl<T> serde::de::Visitor<'_> for Visitor<T>
        where
            T: FromStr,
            T::Err: fmt::Display,
        {
            type Value = FriendlyId<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a friendly route parameter with a leading typed id")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let id_text = value.split_once('-').map_or(value, |(id, _)| id);
                if id_text.is_empty() {
                    return Err(E::custom("missing leading id"));
                }
                let id = id_text.parse::<T>().map_err(E::custom)?;
                Ok(FriendlyId::id_only(id))
            }
        }

        deserializer.deserialize_str(Visitor(std::marker::PhantomData))
    }
}

fn friendly_slug(value: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let mut slug = String::new();
    let mut previous_was_separator = true;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('-');
            previous_was_separator = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.chars().count() <= max_chars {
        return slug;
    }

    let truncated = slug.chars().take(max_chars).collect::<String>();
    if let Some((head, _)) = truncated.rsplit_once('-') {
        if !head.is_empty() {
            return head.to_string();
        }
    }
    truncated.trim_end_matches('-').to_string()
}

#[derive(Clone, Copy, Debug)]
pub struct Path<T>(pub T);

impl<T> std::ops::Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> axum::extract::FromRequestParts<S> for Path<T>
where
    T: serde::de::DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            axum::extract::Path::<T>::from_request_parts(parts, state)
                .await
                .map(|axum::extract::Path(value)| Self(value))
                .map_err(|_| axum::http::StatusCode::NOT_FOUND)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mount {
    pub name: String,
    pub pages: PathBuf,
    pub route_root: String,
    pub module_root: String,
    pub language_param: Option<String>,
    pub handler_name: String,
    pub handler_names: HandlerNames,
    pub router_state_type: Option<String>,
    pub ignored_path_prefixes: Vec<PathBuf>,
    pub include_prefixed_home_route: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandlerNames {
    Fixed(String),
    RouteAction,
}

impl Mount {
    pub fn new(
        name: impl Into<String>,
        pages: impl Into<PathBuf>,
        route_root: impl Into<String>,
        module_root: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            pages: pages.into(),
            route_root: route_root.into(),
            module_root: module_root.into(),
            language_param: None,
            handler_name: "handler".to_string(),
            handler_names: HandlerNames::Fixed("handler".to_string()),
            router_state_type: None,
            ignored_path_prefixes: Vec::new(),
            include_prefixed_home_route: true,
        }
    }

    pub fn with_language_param(mut self, language_param: impl Into<String>) -> Self {
        self.language_param = Some(language_param.into());
        self
    }

    pub fn with_handler_name(mut self, handler_name: impl Into<String>) -> Self {
        let handler_name = handler_name.into();
        self.handler_name = handler_name.clone();
        self.handler_names = HandlerNames::Fixed(handler_name);
        self
    }

    pub fn with_route_action_handler_names(mut self) -> Self {
        self.handler_names = HandlerNames::RouteAction;
        self
    }

    pub fn with_router_state_type(mut self, router_state_type: impl Into<String>) -> Self {
        self.router_state_type = Some(router_state_type.into());
        self
    }

    pub fn with_ignored_path_prefixes(
        mut self,
        prefixes: impl IntoIterator<Item = impl Into<PathBuf>>,
    ) -> Self {
        self.ignored_path_prefixes = prefixes.into_iter().map(Into::into).collect();
        self
    }

    pub fn without_prefixed_home_route(mut self) -> Self {
        self.include_prefixed_home_route = false;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MountRoutes {
    pub mount: Mount,
    pub routes: Vec<Route>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    pub kind: RouteKind,
    pub endpoint: Endpoint,
    pub method: HttpMethod,
    pub name: String,
    pub helper_name: String,
    pub path: String,
    pub segments: Vec<RouteSegment>,
    pub params: Vec<RouteParam>,
    pub source_file: PathBuf,
    pub module_path: String,
    pub handler_name: String,
    pub handler_path: String,
    pub contract: Option<RouteContract>,
}

impl Route {
    pub fn pattern_path(&self) -> String {
        pattern_path(&self.path)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteContract {
    pub type_path: String,
    pub fields: Vec<RouteContractField>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteContractField {
    pub name: String,
    pub type_name: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => f.write_str("GET"),
            HttpMethod::Post => f.write_str("POST"),
            HttpMethod::Delete => f.write_str("DELETE"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Endpoint {
    Page,
    Action(Action),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RouteKind {
    Home,
    NotFound,
    Static,
    Dynamic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RouteSegment {
    Static(String),
    Dynamic(String),
    CatchAll(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteParam {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DiscoverError {
    PagesDirectoryUnreadable {
        path: PathBuf,
    },
    InvalidPageModulePath {
        source_file: PathBuf,
    },
    InvalidPageSegment {
        source_file: PathBuf,
        segment: String,
    },
    ReservedPageSegment {
        source_file: PathBuf,
        segment: String,
    },
    InvalidRouteParam {
        source_file: PathBuf,
        param: String,
    },
    DuplicateRouteParam {
        source_file: PathBuf,
        param: String,
    },
    InvalidRouteName {
        source_file: PathBuf,
        name: String,
    },
    DuplicateRoute {
        method: HttpMethod,
        path: String,
        first_file: PathBuf,
        second_file: PathBuf,
    },
    DuplicateRouteName {
        name: String,
        first_file: PathBuf,
        second_file: PathBuf,
    },
    DuplicateHelper {
        helper: String,
        first_file: PathBuf,
        second_file: PathBuf,
    },
    InvalidMountName {
        mount_name: String,
    },
    PageFileUnreadable {
        source_file: PathBuf,
    },
    MissingHandler {
        source_file: PathBuf,
        handler_name: String,
    },
    InvalidRouteContract {
        source_file: PathBuf,
        message: String,
    },
}

impl fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoverError::PagesDirectoryUnreadable { path } => {
                write!(f, "could not read pages directory {}", path.display())
            }
            DiscoverError::InvalidPageModulePath { source_file } => {
                write!(f, "invalid page module path {}", source_file.display())
            }
            DiscoverError::InvalidPageSegment {
                source_file,
                segment,
            } => write!(
                f,
                "invalid page path segment {segment:?} in {}",
                source_file.display()
            ),
            DiscoverError::ReservedPageSegment {
                source_file,
                segment,
            } => write!(
                f,
                "reserved page path segment {segment:?} in {}. This name is part of proute's route grammar and cannot be used as a GET page segment.",
                source_file.display()
            ),
            DiscoverError::InvalidRouteParam { source_file, param } => write!(
                f,
                "invalid route parameter {param:?} in {}",
                source_file.display()
            ),
            DiscoverError::DuplicateRouteParam { source_file, param } => write!(
                f,
                "duplicate route parameter {param:?} in {}",
                source_file.display()
            ),
            DiscoverError::InvalidRouteName { source_file, name } => {
                write!(
                    f,
                    "invalid route name {name:?} from {}",
                    source_file.display()
                )
            }
            DiscoverError::DuplicateRoute {
                method,
                path,
                first_file,
                second_file,
            } => write!(
                f,
                "duplicate route {method} {path:?} from {} and {}",
                first_file.display(),
                second_file.display()
            ),
            DiscoverError::DuplicateRouteName {
                name,
                first_file,
                second_file,
            } => write!(
                f,
                "duplicate route name {name:?} from {} and {}",
                first_file.display(),
                second_file.display()
            ),
            DiscoverError::DuplicateHelper {
                helper,
                first_file,
                second_file,
            } => write!(
                f,
                "duplicate route helper {helper:?} from {} and {}",
                first_file.display(),
                second_file.display()
            ),
            DiscoverError::InvalidMountName { mount_name } => {
                write!(f, "invalid mount name {mount_name:?}")
            }
            DiscoverError::PageFileUnreadable { source_file } => {
                write!(f, "could not read page file {}", source_file.display())
            }
            DiscoverError::MissingHandler {
                source_file,
                handler_name,
            } => write!(
                f,
                "missing handler {handler_name:?} in {}. Expected `pub(crate) async fn {handler_name}` or `pub async fn {handler_name}`.",
                source_file.display()
            ),
            DiscoverError::InvalidRouteContract {
                source_file,
                message,
            } => write!(
                f,
                "invalid route contract in {}: {message}",
                source_file.display()
            ),
        }
    }
}

impl std::error::Error for DiscoverError {}

pub fn discover_mount(mount: Mount) -> Result<MountRoutes, DiscoverError> {
    if !is_valid_label(&mount.name) {
        return Err(DiscoverError::InvalidMountName {
            mount_name: mount.name.clone(),
        });
    }
    if !is_valid_handler_names(&mount.handler_names) {
        return Err(DiscoverError::InvalidRouteName {
            source_file: mount.pages.clone(),
            name: mount.handler_name.clone(),
        });
    }

    let files = walk_pages(&mount.pages)?;
    let mut routes = Vec::new();

    for file in files {
        if should_ignore_file(&mount, &file) {
            continue;
        }

        routes.push(route_from_file(&mount, &file)?);
    }

    reject_duplicate_routes(&routes)?;
    reject_duplicate_names(&routes)?;
    reject_duplicate_helpers(&routes)?;
    validate_handlers(&routes)?;
    routes.sort_by(route_sort_key);

    Ok(MountRoutes { mount, routes })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug)]
pub enum WriteError {
    Discover(DiscoverError),
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::Discover(error) => error.fmt(f),
            WriteError::Write { path, source } => {
                write!(
                    f,
                    "could not write generated file {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for WriteError {}

pub fn write_mount_file(output_root: &StdPath, mount: Mount) -> Result<GeneratedFile, WriteError> {
    let mount_routes = discover_mount(mount).map_err(WriteError::Discover)?;
    let generated = generate_mount_file(&mount_routes);
    write_generated_file(output_root, &generated)?;

    Ok(generated)
}

pub fn write_mount_files(
    output_root: &StdPath,
    mounts: impl IntoIterator<Item = Mount>,
) -> Result<Vec<GeneratedFile>, WriteError> {
    let mount_routes = mounts
        .into_iter()
        .map(discover_mount)
        .collect::<Result<Vec<_>, _>>()
        .map_err(WriteError::Discover)?;

    let mut generated = mount_routes
        .iter()
        .map(generate_mount_file)
        .collect::<Vec<_>>();
    generated.push(generate_routes_mod_file(&mount_routes));

    for file in &generated {
        write_generated_file(output_root, file)?;
    }

    Ok(generated)
}

fn write_generated_file(
    output_root: &StdPath,
    generated: &GeneratedFile,
) -> Result<(), WriteError> {
    let output_path = output_root.join(&generated.path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|source| WriteError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(&output_path, &generated.contents).map_err(|source| WriteError::Write {
        path: output_path,
        source,
    })?;

    Ok(())
}

pub fn generate_mount_file(mount_routes: &MountRoutes) -> GeneratedFile {
    GeneratedFile {
        path: PathBuf::from("proute").join(format!("{}.rs", mount_routes.mount.name)),
        contents: generate_mount_module(mount_routes),
    }
}

pub fn generate_routes_mod_file(mount_routes: &[MountRoutes]) -> GeneratedFile {
    let modules = mount_routes
        .iter()
        .map(|mount_routes| format!("pub mod {};", mount_routes.mount.name))
        .collect::<Vec<_>>()
        .join("\n");

    GeneratedFile {
        path: PathBuf::from("proute").join("mod.rs"),
        contents: format!("//// Generated. Do not edit.\n\n{modules}\n"),
    }
}

pub fn generate_mount_module(mount_routes: &MountRoutes) -> String {
    let mut sections = Vec::new();
    sections.push(generated_header(mount_routes));
    sections.push(route_spec_type());
    sections.push(route_table(mount_routes));
    sections.push(router_functions(mount_routes));
    sections.push(path_helpers(mount_routes));
    sections.push(percent_encode_function());

    sections.join("\n")
}

fn generated_header(mount_routes: &MountRoutes) -> String {
    format!(
        "//// Generated. Do not edit.\n////\n//// mount: {}\n//// pages: {}\n//// route_root: {}\n//// language_param: {}\n",
        mount_routes.mount.name,
        mount_routes.mount.pages.display(),
        mount_routes.mount.route_root,
        mount_routes
            .mount
            .language_param
            .as_deref()
            .unwrap_or("none")
    )
}

fn route_spec_type() -> String {
    r#"#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteSpec {
    pub method: &'static str,
    pub path: &'static str,
    pub prefixed_path: Option<&'static str>,
    pub name: &'static str,
    pub helper_name: &'static str,
    pub module_path: &'static str,
    pub handler_path: &'static str,
}
"#
    .to_string()
}

fn route_table(mount_routes: &MountRoutes) -> String {
    let specs = mount_routes
        .routes
        .iter()
        .map(|route| {
            format!(
                "RouteSpec {{ method: {:?}, path: {:?}, prefixed_path: {}, name: {:?}, helper_name: {:?}, module_path: {:?}, handler_path: {:?} }},",
                route.method.to_string(),
                route.path,
                optional_string_literal(prefixed_route_path(&mount_routes.mount, &route.path).as_deref()),
                route.name,
                route.helper_name,
                route.module_path,
                route.handler_path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "pub const ROUTES: &[RouteSpec] = &[\n{}\n];\n",
        indent_lines(&specs, 4)
    )
}

fn router_functions(mount_routes: &MountRoutes) -> String {
    let Some(state_type) = mount_routes.mount.router_state_type.as_deref() else {
        return String::new();
    };

    let canonical = router_function("routes", state_type, &route_groups(mount_routes, false));
    if mount_routes.mount.language_param.is_none() {
        return canonical;
    }

    let prefixed = router_function(
        "prefixed_routes",
        state_type,
        &route_groups(mount_routes, true),
    );

    format!("{canonical}\n{prefixed}")
}

fn router_function(name: &str, state_type: &str, groups: &[RouteGroup]) -> String {
    let routes = groups
        .iter()
        .map(route_group_line)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "pub fn {name}() -> axum::Router<{state_type}> {{\n    axum::Router::new()\n{}\n}}\n",
        indent_lines(&routes, 8)
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RouteGroup {
    path: String,
    routes: Vec<Route>,
}

fn route_groups(mount_routes: &MountRoutes, prefixed: bool) -> Vec<RouteGroup> {
    let mut groups: BTreeMap<String, Vec<Route>> = BTreeMap::new();

    for route in &mount_routes.routes {
        if prefixed
            && !mount_routes.mount.include_prefixed_home_route
            && route.kind == RouteKind::Home
        {
            continue;
        }

        let path = if prefixed {
            prefixed_route_path(&mount_routes.mount, &route.path)
                .unwrap_or_else(|| route.path.clone())
        } else {
            route.path.clone()
        };

        groups.entry(path).or_default().push(route.clone());
    }

    groups
        .into_iter()
        .map(|(path, routes)| RouteGroup { path, routes })
        .collect()
}

fn route_group_line(group: &RouteGroup) -> String {
    format!(
        ".route({:?}, {})",
        group.path,
        method_router_expression(&group.routes)
    )
}

fn method_router_expression(routes: &[Route]) -> String {
    let mut routes = routes.to_vec();
    routes.sort_by_key(|route| route.method);

    let mut iter = routes.into_iter();
    let first = iter
        .next()
        .expect("route group must contain at least one route");
    let mut expression = format!(
        "axum::routing::{}({})",
        method_router_fn(first.method),
        first.handler_path
    );

    for route in iter {
        expression.push_str(&format!(
            ".{}({})",
            method_router_fn(route.method),
            route.handler_path
        ));
    }

    expression
}

fn method_router_fn(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Delete => "delete",
    }
}

fn path_helpers(mount_routes: &MountRoutes) -> String {
    mount_routes
        .routes
        .iter()
        .flat_map(|route| path_helpers_for_route(&mount_routes.mount, route))
        .collect::<Vec<_>>()
        .join("\n")
}

fn percent_encode_function() -> String {
    r#"fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();

    for byte in value.bytes() {
        if is_unreserved(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

fn percent_encode_path(value: &str) -> String {
    value
        .split('/')
        .map(percent_encode)
        .collect::<Vec<_>>()
        .join("/")
}

fn is_unreserved(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
    )
}
"#
    .to_string()
}

fn path_helpers_for_route(mount: &Mount, route: &Route) -> Vec<String> {
    let mut helpers = vec![path_helper(route, &route.helper_name, &route.path)];

    if let Some(language_param) = mount.language_param.as_deref() {
        let prefixed_name = prefixed_helper_name(&route.helper_name);
        let prefixed_path =
            prefixed_route_path(mount, &route.path).unwrap_or_else(|| route.path.clone());
        helpers.push(prefixed_path_helper(
            route,
            &prefixed_name,
            language_param,
            &prefixed_path,
        ));

        let localized_name = localized_helper_name(&route.helper_name);
        helpers.push(localized_path_helper(
            &localized_name,
            &prefixed_name,
            route,
            language_param,
        ));
    }

    helpers
}

fn prefixed_helper_name(helper_name: &str) -> String {
    format!("prefixed_{helper_name}")
}

fn localized_helper_name(helper_name: &str) -> String {
    format!("localized_{helper_name}")
}

fn path_helper(route: &Route, helper_name: &str, path: &str) -> String {
    if let Some(contract) = &route.contract {
        let args = contract_helper_params(contract);
        let expression = typed_path_expression_from_template(path, contract);
        return format!(
            "#[allow(non_snake_case)]\npub fn {helper_name}({args}) -> String {{\n    {expression}\n}}\n"
        );
    }

    if route.params.is_empty() {
        format!(
            "#[allow(non_snake_case)]\npub const fn {helper_name}() -> &'static str {{\n    {path:?}\n}}\n"
        )
    } else {
        let args = route_helper_params(route);
        let expression = path_expression_from_template(path);

        format!(
            "#[allow(non_snake_case)]\npub fn {helper_name}({args}) -> String {{\n    {expression}\n}}\n"
        )
    }
}

fn prefixed_path_helper(
    route: &Route,
    helper_name: &str,
    language_param: &str,
    path: &str,
) -> String {
    if let Some(contract) = &route.contract {
        let args = prefixed_contract_helper_params(contract, language_param);
        let expression = typed_path_expression_from_template(path, contract);
        return format!(
            "#[allow(non_snake_case)]\npub fn {helper_name}({args}) -> String {{\n    {expression}\n}}\n"
        );
    }

    let mut params = vec![RouteParam {
        name: language_param.to_string(),
        type_name: "String".to_string(),
    }];
    params.extend(route.params.clone());
    let args = helper_params(&params);
    let expression = path_expression_from_template(path);

    format!(
        "#[allow(non_snake_case)]\npub fn {helper_name}({args}) -> String {{\n    {expression}\n}}\n"
    )
}

fn localized_path_helper(
    localized_name: &str,
    prefixed_name: &str,
    route: &Route,
    language_param: &str,
) -> String {
    let canonical_expression = if let Some(contract) = &route.contract {
        let canonical_args = contract_helper_args(contract);
        format!("{}({canonical_args}).to_string()", route.helper_name)
    } else if route.params.is_empty() {
        format!("{}().to_string()", route.helper_name)
    } else {
        let canonical_args = helper_args(&route.params);
        format!("{}({canonical_args})", route.helper_name)
    };
    let prefixed_args = if let Some(contract) = &route.contract {
        let mut args = vec![language_param.to_string()];
        args.extend(contract_helper_args_list(contract));
        args.join(", ")
    } else {
        let mut params = vec![RouteParam {
            name: language_param.to_string(),
            type_name: "String".to_string(),
        }];
        params.extend(route.params.clone());
        helper_args(&params)
    };
    let args = localized_helper_params(route, language_param);

    format!(
        "#[allow(non_snake_case)]\npub fn {localized_name}({args}) -> String {{\n    if {language_param} == primary_lang {{\n        {canonical_expression}\n    }} else {{\n        {prefixed_name}({prefixed_args})\n    }}\n}}\n"
    )
}

fn contract_helper_params(contract: &RouteContract) -> String {
    contract
        .fields
        .iter()
        .map(|field| {
            format!(
                "{}: impl proute::IntoParam<{}>",
                field.name, field.type_name
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn prefixed_contract_helper_params(contract: &RouteContract, language_param: &str) -> String {
    let mut params = vec![format!("{language_param}: impl std::fmt::Display")];
    let contract_params = contract_helper_params(contract);
    if !contract_params.is_empty() {
        params.push(contract_params);
    }
    params.join(", ")
}

fn helper_params(params: &[RouteParam]) -> String {
    params
        .iter()
        .map(|param| format!("{}: impl std::fmt::Display", param.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn route_helper_params(route: &Route) -> String {
    route
        .params
        .iter()
        .map(|param| format!("{}: impl std::fmt::Display", param.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn localized_helper_params(route: &Route, language_param: &str) -> String {
    let mut params = vec![
        format!("{language_param}: &str"),
        "primary_lang: &str".to_string(),
    ];

    if let Some(contract) = &route.contract {
        params.push(contract_helper_params(contract));
    } else {
        params.extend(
            route
                .params
                .iter()
                .map(|param| format!("{}: impl std::fmt::Display", param.name)),
        );
    }

    params.join(", ")
}

fn helper_args(params: &[RouteParam]) -> String {
    params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn contract_helper_args(contract: &RouteContract) -> String {
    contract_helper_args_list(contract).join(", ")
}

fn contract_helper_args_list(contract: &RouteContract) -> Vec<String> {
    contract
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect()
}

fn path_expression_from_template(path: &str) -> String {
    let mut params = Vec::new();
    let template = path
        .split('/')
        .map(|segment| {
            if let Some(param) = route_template_param(segment) {
                params.push(param);
                "{}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    let encoded_params = params
        .iter()
        .map(|param| {
            if param.catch_all {
                format!("percent_encode_path(&{}.to_string())", param.name)
            } else {
                format!("percent_encode(&{}.to_string())", param.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("format!({template:?}, {encoded_params})")
}

fn typed_path_expression_from_template(path: &str, contract: &RouteContract) -> String {
    let mut params = Vec::new();
    let template = path
        .split('/')
        .map(|segment| {
            if let Some(param) = route_template_param(segment) {
                params.push(param);
                "{}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    let encoded_params = params
        .iter()
        .map(|param| {
            let serialized = if let Some(field) = contract
                .fields
                .iter()
                .find(|field| field.name == param.name)
            {
                format!(
                    "proute::IntoParam::<{}>::into_param({})",
                    field.type_name, param.name
                )
            } else {
                format!("{}.to_string()", param.name)
            };

            if param.catch_all {
                format!("percent_encode_path(&{serialized})")
            } else {
                format!("percent_encode(&{serialized})")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("format!({template:?}, {encoded_params})")
}

struct TemplateParam {
    name: String,
    catch_all: bool,
}

fn route_template_param(segment: &str) -> Option<TemplateParam> {
    let param = segment.strip_prefix('{')?.strip_suffix('}')?;
    let (name, catch_all) = match param.strip_prefix('*') {
        Some(name) => (name, true),
        None => (param, false),
    };
    Some(TemplateParam {
        name: name.to_string(),
        catch_all,
    })
}

fn prefixed_route_path(mount: &Mount, path: &str) -> Option<String> {
    let language_param = mount.language_param.as_deref()?;
    Some(if path == "/" {
        format!("/{{{language_param}}}")
    } else {
        format!("/{{{language_param}}}{path}")
    })
}

fn optional_string_literal(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("Some({value:?})"),
        None => "None".to_string(),
    }
}

fn indent_lines(value: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    value
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn walk_pages(root: &StdPath) -> Result<Vec<PathBuf>, DiscoverError> {
    let entries = fs::read_dir(root).map_err(|_| DiscoverError::PagesDirectoryUnreadable {
        path: root.to_path_buf(),
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| DiscoverError::PagesDirectoryUnreadable {
            path: root.to_path_buf(),
        })?;
        let path = entry.path();

        if path.is_dir() {
            paths.extend(walk_pages(&path)?);
        } else {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn should_ignore_file(mount: &Mount, file: &StdPath) -> bool {
    if file.extension().is_none_or(|extension| extension != "rs") {
        return true;
    }

    if file.file_name().is_some_and(|name| name == "mod.rs") {
        return true;
    }

    let Ok(relative) = file.strip_prefix(&mount.pages) else {
        return true;
    };

    if mount
        .ignored_path_prefixes
        .iter()
        .any(|prefix| relative.starts_with(prefix))
    {
        return true;
    }

    relative_parts(&mount.pages, file).is_ok_and(|parts| parts.iter().any(|part| part == "shared"))
}

fn route_from_file(mount: &Mount, source_file: &StdPath) -> Result<Route, DiscoverError> {
    let raw_segments = raw_segments(mount, source_file)?;

    validate_raw_segments(source_file, &raw_segments)?;
    validate_file_is_not_namespace_parent(source_file, &raw_segments)?;

    let module_path = module_path(mount, source_file)?;

    if raw_segments
        .last()
        .is_some_and(|segment| segment == "not_found_")
    {
        return Ok(not_found_route(mount, source_file, module_path));
    }

    let endpoint = endpoint_for(&raw_segments);
    let route_segments = route_segments(&raw_segments, &endpoint);
    validate_catch_all_position(source_file, &route_segments)?;
    let params = dynamic_params(&route_segments);
    validate_params(source_file, &params)?;

    let name = route_name(&raw_segments);
    if name.is_empty() {
        return Err(DiscoverError::InvalidRouteName {
            source_file: source_file.to_path_buf(),
            name,
        });
    }

    let handler_name = handler_name_for(mount, &raw_segments);
    let handler_path = format!("{module_path}::{handler_name}");
    let contract = route_contract(source_file, &module_path, &handler_name, &params)?;

    Ok(Route {
        kind: route_kind(&route_segments),
        method: method_for(&endpoint),
        endpoint,
        helper_name: helper_name(&raw_segments),
        path: route_path(&mount.route_root, &route_segments),
        segments: route_segments,
        params,
        source_file: source_file.to_path_buf(),
        module_path,
        handler_name,
        handler_path,
        name,
        contract,
    })
}

fn raw_segments(mount: &Mount, source_file: &StdPath) -> Result<Vec<String>, DiscoverError> {
    let relative = source_file.strip_prefix(&mount.pages).map_err(|_| {
        DiscoverError::InvalidPageModulePath {
            source_file: source_file.to_path_buf(),
        }
    })?;

    let mut segments = Vec::new();
    for component in relative.components() {
        let segment = component.as_os_str().to_string_lossy();
        let segment = segment.strip_suffix(".rs").unwrap_or(&segment);
        segments.push(segment.to_string());
    }

    Ok(segments)
}

fn endpoint_for(raw_segments: &[String]) -> Endpoint {
    match raw_segments.last().map(String::as_str) {
        Some("create") => Endpoint::Action(Action::Create),
        Some("update") => Endpoint::Action(Action::Update),
        Some("delete") => Endpoint::Action(Action::Delete),
        _ => Endpoint::Page,
    }
}

fn method_for(endpoint: &Endpoint) -> HttpMethod {
    match endpoint {
        Endpoint::Page => HttpMethod::Get,
        Endpoint::Action(Action::Create | Action::Update | Action::Delete) => HttpMethod::Post,
    }
}

fn route_segments(raw_segments: &[String], endpoint: &Endpoint) -> Vec<RouteSegment> {
    let path_segments = match endpoint {
        Endpoint::Page => raw_segments,
        Endpoint::Action(Action::Create | Action::Update) => {
            &raw_segments[..raw_segments.len() - 1]
        }
        Endpoint::Action(Action::Delete) => raw_segments,
    };
    let path_segments = match path_segments.last().map(String::as_str) {
        Some("index") => &path_segments[..path_segments.len() - 1],
        _ => path_segments,
    };

    path_segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if index + 1 == path_segments.len() {
                route_leaf_segment(segment)
            } else {
                route_segment(segment)
            }
        })
        .collect()
}

fn route_leaf_segment(segment: &str) -> RouteSegment {
    match segment {
        "export" => RouteSegment::Static("export.csv".to_string()),
        "items" => RouteSegment::Static("items.json".to_string()),
        _ => route_segment(segment),
    }
}

fn route_segment(segment: &str) -> RouteSegment {
    if segment == "all_" {
        RouteSegment::CatchAll("all".to_string())
    } else if let Some(param) = segment.strip_suffix('_') {
        RouteSegment::Dynamic(param.to_string())
    } else {
        RouteSegment::Static(segment.to_string())
    }
}

fn validate_catch_all_position(
    source_file: &StdPath,
    segments: &[RouteSegment],
) -> Result<(), DiscoverError> {
    let Some(index) = segments
        .iter()
        .position(|segment| matches!(segment, RouteSegment::CatchAll(_)))
    else {
        return Ok(());
    };

    if index + 1 == segments.len() {
        Ok(())
    } else {
        Err(DiscoverError::InvalidPageSegment {
            source_file: source_file.to_path_buf(),
            segment: "all_".to_string(),
        })
    }
}

fn route_kind(segments: &[RouteSegment]) -> RouteKind {
    if segments.is_empty() {
        return RouteKind::Home;
    }

    if segments.iter().any(|segment| {
        matches!(
            segment,
            RouteSegment::Dynamic(_) | RouteSegment::CatchAll(_)
        )
    }) {
        RouteKind::Dynamic
    } else {
        RouteKind::Static
    }
}

fn dynamic_params(segments: &[RouteSegment]) -> Vec<RouteParam> {
    segments
        .iter()
        .filter_map(|segment| match segment {
            RouteSegment::Static(_) => None,
            RouteSegment::Dynamic(name) | RouteSegment::CatchAll(name) => Some(RouteParam {
                name: name.clone(),
                type_name: "String".to_string(),
            }),
        })
        .collect()
}

fn route_path(route_root: &str, segments: &[RouteSegment]) -> String {
    let suffix = segments
        .iter()
        .map(route_segment_path)
        .collect::<Vec<_>>()
        .join("/");

    match (normalize_root(route_root).as_str(), suffix.as_str()) {
        ("/", "") => "/".to_string(),
        ("/", suffix) => format!("/{suffix}"),
        (root, "") => root.to_string(),
        (root, suffix) => format!("{root}/{suffix}"),
    }
}

fn route_segment_path(segment: &RouteSegment) -> String {
    match segment {
        RouteSegment::Static(value) => value.clone(),
        RouteSegment::Dynamic(name) => format!("{{{name}}}"),
        RouteSegment::CatchAll(name) => format!("{{*{name}}}"),
    }
}

fn pattern_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{_}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn not_found_route(mount: &Mount, source_file: &StdPath, module_path: String) -> Route {
    let segments = vec![RouteSegment::Static("not_found".to_string())];
    let handler_name = handler_name_for(mount, &["not_found_".to_string()]);
    let handler_path = format!("{module_path}::{handler_name}");

    Route {
        kind: RouteKind::NotFound,
        endpoint: Endpoint::Page,
        method: HttpMethod::Get,
        name: "not_found_".to_string(),
        helper_name: "not_found".to_string(),
        path: route_path(&mount.route_root, &segments),
        segments,
        params: Vec::new(),
        source_file: source_file.to_path_buf(),
        module_path,
        handler_name,
        handler_path,
        contract: None,
    }
}

fn is_valid_handler_names(handler_names: &HandlerNames) -> bool {
    match handler_names {
        HandlerNames::Fixed(handler_name) => is_valid_label(handler_name),
        HandlerNames::RouteAction => true,
    }
}

fn handler_name_for(mount: &Mount, raw_segments: &[String]) -> String {
    match &mount.handler_names {
        HandlerNames::Fixed(handler_name) => handler_name.clone(),
        HandlerNames::RouteAction => raw_segments
            .last()
            .map(|segment| route_action_handler_name(segment))
            .unwrap_or_else(|| "index".to_string()),
    }
}

fn route_action_handler_name(segment: &str) -> String {
    match segment {
        "export" => "export_csv".to_string(),
        "items" => "items_json".to_string(),
        _ => segment.trim_end_matches('_').to_string(),
    }
}

fn route_name(raw_segments: &[String]) -> String {
    let name = raw_segments
        .iter()
        .filter(|segment| segment.as_str() != "index")
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("/");

    if name.is_empty() {
        "home".to_string()
    } else {
        name
    }
}

fn helper_name(raw_segments: &[String]) -> String {
    let helper = raw_segments
        .iter()
        .filter(|segment| segment.as_str() != "index")
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("_");

    if helper.is_empty() {
        "home".to_string()
    } else {
        helper
    }
}

fn module_path(mount: &Mount, source_file: &StdPath) -> Result<String, DiscoverError> {
    let relative = source_file.strip_prefix(&mount.pages).map_err(|_| {
        DiscoverError::InvalidPageModulePath {
            source_file: source_file.to_path_buf(),
        }
    })?;

    let mut parts = vec![mount.module_root.clone()];
    for component in relative.components() {
        let segment = component.as_os_str().to_string_lossy();
        let segment = segment.strip_suffix(".rs").unwrap_or(&segment);
        parts.push(segment.to_string());
    }

    Ok(parts.join("::"))
}

fn validate_raw_segments(
    source_file: &StdPath,
    raw_segments: &[String],
) -> Result<(), DiscoverError> {
    for (index, segment) in raw_segments.iter().enumerate() {
        if !is_valid_module_segment(segment) {
            return Err(DiscoverError::InvalidPageSegment {
                source_file: source_file.to_path_buf(),
                segment: segment.clone(),
            });
        }

        if is_reserved_page_segment(raw_segments, index) {
            return Err(DiscoverError::ReservedPageSegment {
                source_file: source_file.to_path_buf(),
                segment: segment.clone(),
            });
        }
    }

    Ok(())
}

fn validate_file_is_not_namespace_parent(
    source_file: &StdPath,
    raw_segments: &[String],
) -> Result<(), DiscoverError> {
    let Some(segment) = raw_segments.last() else {
        return Ok(());
    };

    if matches!(
        segment.as_str(),
        "index" | "not_found_" | "create" | "update" | "delete"
    ) {
        return Ok(());
    }

    if source_file.with_extension("").is_dir() {
        return Err(DiscoverError::ReservedPageSegment {
            source_file: source_file.to_path_buf(),
            segment: segment.clone(),
        });
    }

    Ok(())
}

fn is_reserved_page_segment(raw_segments: &[String], index: usize) -> bool {
    let segment = raw_segments[index].as_str();
    let is_last = index == raw_segments.len() - 1;

    match segment {
        "index" => !is_last,
        "show" => true,
        segment
            if segment.ends_with('_') && is_last && !matches!(segment, "not_found_" | "all_") =>
        {
            true
        }
        "create" | "update" | "delete" => !is_last,
        _ => false,
    }
}

fn validate_params(source_file: &StdPath, params: &[RouteParam]) -> Result<(), DiscoverError> {
    let mut seen = BTreeSet::new();

    for param in params {
        if !is_valid_label(&param.name) {
            return Err(DiscoverError::InvalidRouteParam {
                source_file: source_file.to_path_buf(),
                param: param.name.clone(),
            });
        }

        if !seen.insert(param.name.clone()) {
            return Err(DiscoverError::DuplicateRouteParam {
                source_file: source_file.to_path_buf(),
                param: param.name.clone(),
            });
        }
    }

    Ok(())
}

fn route_contract(
    source_file: &StdPath,
    module_path: &str,
    handler_name: &str,
    params: &[RouteParam],
) -> Result<Option<RouteContract>, DiscoverError> {
    if params.is_empty() {
        return Ok(None);
    }

    let source =
        fs::read_to_string(source_file).map_err(|_| DiscoverError::PageFileUnreadable {
            source_file: source_file.to_path_buf(),
        })?;
    let parsed = syn::parse_file(&source).map_err(|error| DiscoverError::InvalidRouteContract {
        source_file: source_file.to_path_buf(),
        message: format!("could not parse Rust source: {error}"),
    })?;

    let Some(item_struct) = parsed.items.iter().find_map(|item| match item {
        syn::Item::Struct(item_struct) if item_struct.ident == "RouteParams" => Some(item_struct),
        _ => None,
    }) else {
        return Ok(None);
    };

    if !is_generated_visible(&item_struct.vis) {
        return Err(DiscoverError::InvalidRouteContract {
            source_file: source_file.to_path_buf(),
            message: "`RouteParams` must be `pub(crate)` or `pub` so generated helpers can use it"
                .to_string(),
        });
    }

    let syn::Fields::Named(fields) = &item_struct.fields else {
        return Err(DiscoverError::InvalidRouteContract {
            source_file: source_file.to_path_buf(),
            message: "`RouteParams` must use named fields".to_string(),
        });
    };

    let mut contract_fields = Vec::new();
    for field in &fields.named {
        if !is_generated_visible(&field.vis) {
            return Err(DiscoverError::InvalidRouteContract {
                source_file: source_file.to_path_buf(),
                message: "`RouteParams` fields must be `pub(crate)` or `pub` so generated helpers can use them".to_string(),
            });
        }

        let Some(ident) = &field.ident else {
            return Err(DiscoverError::InvalidRouteContract {
                source_file: source_file.to_path_buf(),
                message: "`RouteParams` must use named fields".to_string(),
            });
        };

        contract_fields.push(RouteContractField {
            name: ident.to_string(),
            type_name: field.ty.to_token_stream().to_string(),
        });
    }

    validate_route_contract_fields(source_file, params, &contract_fields)?;

    if !handler_receives_route_params(&parsed, handler_name) {
        return Err(DiscoverError::InvalidRouteContract {
            source_file: source_file.to_path_buf(),
            message: format!(
                "`RouteParams` is declared, so handler `{handler_name}` must receive `proute::Path<RouteParams>`"
            ),
        });
    }

    Ok(Some(RouteContract {
        type_path: format!("{module_path}::RouteParams"),
        fields: contract_fields,
    }))
}

fn handler_receives_route_params(parsed: &syn::File, handler_name: &str) -> bool {
    let Some(function) = parsed.items.iter().find_map(|item| match item {
        syn::Item::Fn(function) if function.sig.ident == handler_name => Some(function),
        _ => None,
    }) else {
        return false;
    };

    function.sig.inputs.iter().any(|arg| match arg {
        syn::FnArg::Typed(pat_type) => type_is_proute_route_params_path(&pat_type.ty),
        syn::FnArg::Receiver(_) => false,
    })
}

fn type_is_proute_route_params_path(ty: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };

    let mut segments = type_path.path.segments.iter().collect::<Vec<_>>();
    let Some(last) = segments.pop() else {
        return false;
    };
    let Some(previous) = segments.pop() else {
        return false;
    };

    if previous.ident != "proute" || last.ident != "Path" {
        return false;
    }

    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return false;
    };

    args.args.iter().any(|arg| match arg {
        syn::GenericArgument::Type(syn::Type::Path(inner)) => inner
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "RouteParams"),
        _ => false,
    })
}

fn validate_route_contract_fields(
    source_file: &StdPath,
    params: &[RouteParam],
    fields: &[RouteContractField],
) -> Result<(), DiscoverError> {
    let param_names = params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<BTreeSet<_>>();
    let field_names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();

    if param_names == field_names {
        return Ok(());
    }

    let missing = param_names
        .difference(&field_names)
        .copied()
        .collect::<Vec<_>>();
    let extra = field_names
        .difference(&param_names)
        .copied()
        .collect::<Vec<_>>();
    let mut parts = Vec::new();

    if !missing.is_empty() {
        parts.push(format!("missing fields for params: {}", missing.join(", ")));
    }

    if !extra.is_empty() {
        parts.push(format!(
            "extra fields without route params: {}",
            extra.join(", ")
        ));
    }

    Err(DiscoverError::InvalidRouteContract {
        source_file: source_file.to_path_buf(),
        message: parts.join("; "),
    })
}

fn is_generated_visible(vis: &syn::Visibility) -> bool {
    matches!(
        vis,
        syn::Visibility::Public(_) | syn::Visibility::Restricted(_)
    )
}

fn is_valid_module_segment(segment: &str) -> bool {
    is_valid_label(segment)
}

fn is_valid_label(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_lowercase()
        && chars.all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '_')
        && !is_reserved_rust_word(name)
}

fn is_reserved_rust_word(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}

fn reject_duplicate_routes(routes: &[Route]) -> Result<(), DiscoverError> {
    let mut seen: BTreeMap<(HttpMethod, String), &Route> = BTreeMap::new();

    for route in routes {
        let key = (route.method, route.pattern_path());
        if let Some(first) = seen.get(&key) {
            return Err(DiscoverError::DuplicateRoute {
                method: route.method,
                path: key.1,
                first_file: first.source_file.clone(),
                second_file: route.source_file.clone(),
            });
        }

        seen.insert(key, route);
    }

    Ok(())
}

fn reject_duplicate_names(routes: &[Route]) -> Result<(), DiscoverError> {
    let mut seen: BTreeMap<String, &Route> = BTreeMap::new();

    for route in routes {
        if let Some(first) = seen.get(&route.name) {
            return Err(DiscoverError::DuplicateRouteName {
                name: route.name.clone(),
                first_file: first.source_file.clone(),
                second_file: route.source_file.clone(),
            });
        }

        seen.insert(route.name.clone(), route);
    }

    Ok(())
}

fn reject_duplicate_helpers(routes: &[Route]) -> Result<(), DiscoverError> {
    let mut seen: BTreeMap<String, &Route> = BTreeMap::new();

    for route in routes {
        if !is_valid_label(&route.helper_name) {
            return Err(DiscoverError::InvalidRouteName {
                source_file: route.source_file.clone(),
                name: route.helper_name.clone(),
            });
        }

        if let Some(first) = seen.get(&route.helper_name) {
            return Err(DiscoverError::DuplicateHelper {
                helper: route.helper_name.clone(),
                first_file: first.source_file.clone(),
                second_file: route.source_file.clone(),
            });
        }

        seen.insert(route.helper_name.clone(), route);
    }

    Ok(())
}

fn validate_handlers(routes: &[Route]) -> Result<(), DiscoverError> {
    for route in routes {
        let source = fs::read_to_string(&route.source_file).map_err(|_| {
            DiscoverError::PageFileUnreadable {
                source_file: route.source_file.clone(),
            }
        })?;

        if !has_public_handler(&source, &route.handler_name) {
            return Err(DiscoverError::MissingHandler {
                source_file: route.source_file.clone(),
                handler_name: route.handler_name.clone(),
            });
        }
    }

    Ok(())
}

fn has_public_handler(source: &str, handler_name: &str) -> bool {
    let declarations = public_fn_declarations(source);
    declarations.iter().any(|declaration| {
        declaration.starts_with(&format!("pub(crate) async fn {handler_name}"))
            || declaration.starts_with(&format!("pub(crate) fn {handler_name}"))
            || declaration.starts_with(&format!("pub async fn {handler_name}"))
            || declaration.starts_with(&format!("pub fn {handler_name}"))
            || is_public_handler_reexport(declaration, handler_name)
    })
}

fn is_public_handler_reexport(declaration: &str, handler_name: &str) -> bool {
    (declaration.starts_with("pub(crate) use ") || declaration.starts_with("pub use "))
        && (declaration.ends_with(&format!("::{handler_name};"))
            || declaration.ends_with(&format!(" as {handler_name};")))
}

fn public_fn_declarations(source: &str) -> Vec<String> {
    let mut declarations = Vec::new();
    let mut current = String::new();

    for line in source
        .lines()
        .map(strip_line_comment)
        .map(|line| line.trim().to_string())
    {
        if line.is_empty() || line.starts_with("#[") {
            continue;
        }

        if current.is_empty() {
            if !is_public_handler_declaration_start(&line) {
                continue;
            }
            current.push_str(&line);
        } else {
            current.push(' ');
            current.push_str(&line);
        }

        if line.contains('{') || line.contains(';') {
            declarations.push(normalize_whitespace(&current));
            current.clear();
        }
    }

    if !current.is_empty() {
        declarations.push(normalize_whitespace(&current));
    }

    declarations
}

fn is_public_handler_declaration_start(line: &str) -> bool {
    line.starts_with("pub(crate) async fn ")
        || line.starts_with("pub(crate) fn ")
        || line.starts_with("pub async fn ")
        || line.starts_with("pub fn ")
        || line.starts_with("pub(crate) use ")
        || line.starts_with("pub use ")
}

fn strip_line_comment(line: &str) -> String {
    line.split("//").next().unwrap_or(line).to_string()
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn route_sort_key(left: &Route, right: &Route) -> std::cmp::Ordering {
    comparable_route_key(left).cmp(&comparable_route_key(right))
}

fn comparable_route_key(route: &Route) -> (u8, String, HttpMethod, String) {
    let kind = match route.kind {
        RouteKind::Home => 0,
        RouteKind::Static | RouteKind::Dynamic => 1,
        RouteKind::NotFound => 2,
    };
    let segments = route
        .segments
        .iter()
        .map(|segment| match segment {
            RouteSegment::Static(value) => format!("1:{value}"),
            RouteSegment::Dynamic(_) => "2:".to_string(),
            RouteSegment::CatchAll(_) => "3:".to_string(),
        })
        .chain(std::iter::once("0".to_string()))
        .collect::<Vec<_>>()
        .join("/");

    (kind, segments, route.method, route.path.clone())
}

fn normalize_root(route_root: &str) -> String {
    let trimmed = route_root.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn relative_parts(root: &StdPath, file: &StdPath) -> Result<Vec<String>, ()> {
    let relative = file.strip_prefix(root).map_err(|_| ())?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn discovers_proute_routes_with_http_action_overlay() {
        let fixture = Fixture::new("routes");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/new.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_/index.rs");
        fixture.write("orders/order_id_/edit.rs");
        fixture.write("orders/order_id_/update.rs");
        fixture.write("orders/order_id_/delete.rs");
        fixture.write("orders/export.rs");
        fixture.write("orders/items.rs");
        fixture.write("orders/shared/form.rs");
        fixture.write("orders/mod.rs");

        let routes = discover_mount(fixture.mount()).unwrap().routes;
        let table = routes
            .iter()
            .map(|route| {
                format!(
                    "{} {} {} {}",
                    route.method, route.path, route.name, route.module_path
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            table,
            [
                "GET / home crate::pages::index",
                "GET /orders orders crate::pages::orders::index",
                "POST /orders orders/create crate::pages::orders::create",
                "GET /orders/export.csv orders/export crate::pages::orders::export",
                "GET /orders/items.json orders/items crate::pages::orders::items",
                "GET /orders/new orders/new crate::pages::orders::new",
                "GET /orders/{order_id} orders/order_id_ crate::pages::orders::order_id_::index",
                "POST /orders/{order_id} orders/order_id_/update crate::pages::orders::order_id_::update",
                "POST /orders/{order_id}/delete orders/order_id_/delete crate::pages::orders::order_id_::delete",
                "GET /orders/{order_id}/edit orders/order_id_/edit crate::pages::orders::order_id_::edit",
                "GET /not_found not_found_ crate::pages::not_found_",
            ]
        );
    }

    #[test]
    fn supports_mount_roots() {
        let fixture = Fixture::new("mount_roots");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("games/id_/index.rs");

        let mut mount = fixture.mount();
        mount.route_root = "/admin".to_string();
        mount.module_root = "crate::pages::admin".to_string();

        let routes = discover_mount(mount).unwrap().routes;
        let table = routes
            .iter()
            .map(|route| format!("{} {} {}", route.method, route.path, route.module_path))
            .collect::<Vec<_>>();

        assert_eq!(
            table,
            [
                "GET /admin crate::pages::admin::index",
                "GET /admin/games/{id} crate::pages::admin::games::id_::index",
                "GET /admin/not_found crate::pages::admin::not_found_",
            ]
        );
    }

    #[test]
    fn generated_file_targets_proute_directory() {
        let fixture = Fixture::new("generated_path");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/order_id_/index.rs");

        let mount_routes = discover_mount(fixture.mount().with_language_param("lang")).unwrap();
        let generated = generate_mount_file(&mount_routes);

        assert_eq!(generated.path, PathBuf::from("proute/public.rs"));
        assert!(generated.contents.contains("//// mount: public"));
        assert!(generated.contents.contains("pub const ROUTES"));
        assert!(
            generated
                .contents
                .contains("helper_name: \"orders_order_id_\"")
        );
        assert!(
            generated
                .contents
                .contains("pub fn orders_order_id_(order_id: impl std::fmt::Display) -> String")
        );
        assert!(
            generated
                .contents
                .contains("format!(\"/orders/{}\", percent_encode(&order_id.to_string()))")
        );
    }

    #[test]
    fn generated_routes_include_i18n_prefixes_when_mount_requests_them() {
        let fixture = Fixture::new("i18n_routes");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/order_id_/index.rs");

        let mount_routes = discover_mount(fixture.mount().with_language_param("lang")).unwrap();
        let generated = generate_mount_file(&mount_routes);

        assert!(generated.contents.contains("language_param: lang"));
        assert!(
            generated
                .contents
                .contains("prefixed_path: Some(\"/{lang}/orders\")")
        );
        assert!(
            generated
                .contents
                .contains("prefixed_path: Some(\"/{lang}/orders/{order_id}\")")
        );
        assert!(
            generated
                .contents
                .contains("pub fn prefixed_orders_order_id_(lang: impl std::fmt::Display, order_id: impl std::fmt::Display) -> String")
        );
        assert!(
            generated
                .contents
                .contains("pub fn localized_orders_order_id_(lang: &str, primary_lang: &str, order_id: impl std::fmt::Display) -> String")
        );
        assert!(
            generated
                .contents
                .contains("format!(\"/{}/orders\", percent_encode(&lang.to_string()))")
        );
        assert!(
            generated
                .contents
                .contains("format!(\"/{}/orders/{}\", percent_encode(&lang.to_string()), percent_encode(&order_id.to_string()))")
        );
    }

    #[test]
    fn generated_module_compiles_as_rust() {
        let fixture = Fixture::new("generated_compiles");
        fixture.write("index.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/order_id_/index.rs");
        fixture.write("orders/order_id_/update.rs");

        let mount_routes = discover_mount(fixture.mount().with_language_param("lang")).unwrap();
        let source_path = fixture.root.join("generated_public.rs");
        fs::write(&source_path, generate_mount_module(&mount_routes)).unwrap();

        let output = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg("--crate-type=lib")
            .arg(&source_path)
            .arg("-o")
            .arg(fixture.root.join("generated_public.rlib"))
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "generated module did not compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn generated_helpers_use_page_local_route_contracts() {
        let fixture = Fixture::new("typed_helpers");
        fixture.write("index.rs");
        fixture.write_source(
            "orders/order_id_/index.rs",
            r#"
pub(crate) struct RouteParams {
    pub(crate) order_id: i64,
}

pub(crate) async fn handler(proute::Path(params): proute::Path<RouteParams>) {
    let _ = params.order_id;
}
"#,
        );
        fixture.write_source(
            "products/product_type_/index.rs",
            r#"
pub(crate) struct RouteParams {
    pub(crate) product_type: String,
}

pub(crate) async fn handler(proute::Path(params): proute::Path<RouteParams>) {
    let _ = params.product_type;
}
"#,
        );
        fixture.write_source(
            "pages/id_/index.rs",
            r#"
pub(crate) struct RouteParams {
    pub(crate) id: proute::FriendlyId<i64>,
}

pub(crate) async fn handler(proute::Path(params): proute::Path<RouteParams>) {
    let _ = params.id;
}
"#,
        );

        let mount_routes = discover_mount(fixture.mount().with_language_param("lang")).unwrap();
        let generated = generate_mount_module(&mount_routes);

        assert!(
            generated.contains(
                "pub fn orders_order_id_(order_id: impl proute::IntoParam<i64>) -> String"
            )
        );
        assert!(
            generated.contains("percent_encode(&proute::IntoParam::<i64>::into_param(order_id))")
        );
        assert!(generated.contains(
            "pub fn localized_orders_order_id_(lang: &str, primary_lang: &str, order_id: impl proute::IntoParam<i64>) -> String"
        ));

        let source_path = fixture.root.join("generated_typed_helpers.rs");
        fs::write(
            &source_path,
            generated_typed_helper_compile_wrapper(&generated),
        )
        .unwrap();

        let binary_path = fixture.root.join("generated_typed_helpers");
        let compile = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .unwrap();

        assert!(
            compile.status.success(),
            "generated typed helper binary did not compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let run = std::process::Command::new(binary_path).output().unwrap();

        assert!(
            run.status.success(),
            "generated typed helper binary failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }

    #[test]
    fn friendly_id_generates_bounded_slugs_and_parses_leading_id() {
        use serde::Deserialize as _;
        use serde::de::value::{Error as ValueError, StrDeserializer};

        let param = FriendlyId::new(123, "This is a Long Title, With Punctuation and Words")
            .with_slug_limit(24);
        assert_eq!(param.to_param(), "123-this-is-a-long-title");

        let empty = FriendlyId::new(123, "!!!");
        assert_eq!(empty.to_param(), "123");

        let parsed = FriendlyId::<i64>::deserialize(StrDeserializer::<ValueError>::new(
            "123-this-is-a-long-title",
        ))
        .expect("parse friendly id");
        assert_eq!(parsed.id, 123);

        assert!(
            FriendlyId::<i64>::deserialize(StrDeserializer::<ValueError>::new("abc-title"))
                .is_err()
        );
        assert!(
            FriendlyId::<i64>::deserialize(StrDeserializer::<ValueError>::new("-title")).is_err()
        );
    }

    #[test]
    fn route_params_contract_requires_handler_receiver() {
        let fixture = Fixture::new("typed_handler_receiver");
        fixture.write("index.rs");
        fixture.write_source(
            "orders/order_id_/index.rs",
            r#"
pub(crate) struct RouteParams {
    pub(crate) order_id: i64,
}

pub(crate) async fn handler() {}
"#,
        );

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::InvalidRouteContract {
                source_file,
                message,
            } if source_file.ends_with("orders/order_id_/index.rs")
                && message.contains("must receive `proute::Path<RouteParams>`")
        ));
    }

    #[tokio::test]
    async fn path_extractor_maps_typed_param_parse_failures_to_not_found() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        #[derive(serde::Deserialize)]
        struct RouteParams {
            id: i64,
        }

        async fn handler(Path(params): Path<RouteParams>) -> String {
            params.id.to_string()
        }

        let app = axum::Router::new().route("/orders/{id}", axum::routing::get(handler));

        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/orders/123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);

        let bad_param = app
            .oneshot(
                Request::builder()
                    .uri("/orders/nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_param.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn generated_router_module_compiles_with_stub_axum() {
        let fixture = Fixture::new("generated_router_compiles");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_/index.rs");
        fixture.write("orders/order_id_/update.rs");
        fixture.write("orders/order_id_/delete.rs");

        let mount = fixture
            .mount()
            .with_language_param("lang")
            .with_router_state_type("crate::app::AppState");
        let mount_routes = discover_mount(mount).unwrap();
        let generated = generate_mount_module(&mount_routes);
        let source_path = fixture.root.join("generated_router.rs");
        fs::write(&source_path, generated_router_compile_wrapper(&generated)).unwrap();

        let output = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg("--crate-type=lib")
            .arg(&source_path)
            .arg("-o")
            .arg(fixture.root.join("generated_router.rlib"))
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "generated router module did not compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn generated_router_groups_handlers_by_path() {
        let fixture = Fixture::new("generated_router_groups");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_/index.rs");
        fixture.write("orders/order_id_/update.rs");
        fixture.write("orders/order_id_/delete.rs");

        let mount_routes = discover_mount(
            fixture
                .mount()
                .with_language_param("lang")
                .with_router_state_type("crate::app::AppState"),
        )
        .unwrap();
        let generated = generate_mount_module(&mount_routes);

        assert!(generated.contains("pub fn routes() -> axum::Router<crate::app::AppState>"));
        assert!(
            generated.contains("pub fn prefixed_routes() -> axum::Router<crate::app::AppState>")
        );
        assert!(
            generated
                .contains(".route(\"/orders\", axum::routing::get(crate::pages::orders::index::handler).post(crate::pages::orders::create::handler))")
        );
        assert!(
            generated.contains(
                ".route(\"/orders/{order_id}\", axum::routing::get(crate::pages::orders::order_id_::index::handler).post(crate::pages::orders::order_id_::update::handler))"
            )
        );
        assert!(
            generated.contains(
                ".route(\"/orders/{order_id}/delete\", axum::routing::post(crate::pages::orders::order_id_::delete::handler))"
            )
        );
        assert!(
            generated
                .contains(".route(\"/{lang}/orders\", axum::routing::get(crate::pages::orders::index::handler).post(crate::pages::orders::create::handler))")
        );
    }

    #[test]
    fn generated_prefixed_router_can_skip_home_route() {
        let fixture = Fixture::new("generated_router_skips_prefixed_home");
        fixture.write("index.rs");
        fixture.write("cart/index.rs");

        let mount_routes = discover_mount(
            fixture
                .mount()
                .with_language_param("lang")
                .with_router_state_type("crate::app::AppState")
                .without_prefixed_home_route(),
        )
        .unwrap();
        let generated = generate_mount_module(&mount_routes);

        assert!(
            generated.contains(".route(\"/\", axum::routing::get(crate::pages::index::handler))")
        );
        assert!(generated.contains(
            ".route(\"/{lang}/cart\", axum::routing::get(crate::pages::cart::index::handler))"
        ));
        assert!(!generated.contains(".route(\"/{lang}\","));
    }

    #[test]
    fn write_mount_file_writes_under_proute_directory() {
        let fixture = Fixture::new("write_mount_file");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");

        let output_root = fixture.root.join("generated");
        let generated = write_mount_file(&output_root, fixture.mount()).unwrap();
        let output_path = output_root.join("proute/public.rs");

        assert_eq!(generated.path, PathBuf::from("proute/public.rs"));
        assert!(output_path.exists());
        assert!(
            fs::read_to_string(output_path)
                .unwrap()
                .contains("pub const ROUTES")
        );
    }

    #[test]
    fn write_mount_files_writes_mount_modules_and_proute_mod() {
        let fixture = Fixture::new("write_mount_files");
        let public_pages = fixture.root.join("public_pages");
        let admin_pages = fixture.root.join("admin_pages");
        write_fixture_file(&public_pages, "index.rs");
        write_fixture_file(&public_pages, "not_found_.rs");
        write_fixture_file(&admin_pages, "index.rs");
        write_fixture_file(&admin_pages, "not_found_.rs");

        let output_root = fixture.root.join("generated");
        let generated = write_mount_files(
            &output_root,
            [
                Mount::new("public", &public_pages, "/", "crate::pages::public")
                    .with_language_param("lang"),
                Mount::new("admin", &admin_pages, "/admin", "crate::pages::admin")
                    .with_language_param("lang"),
            ],
        )
        .unwrap();

        let paths = generated
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            paths,
            [
                PathBuf::from("proute/public.rs"),
                PathBuf::from("proute/admin.rs"),
                PathBuf::from("proute/mod.rs"),
            ]
        );
        assert!(output_root.join("proute/public.rs").exists());
        assert!(output_root.join("proute/admin.rs").exists());
        assert_eq!(
            fs::read_to_string(output_root.join("proute/mod.rs")).unwrap(),
            "//// Generated. Do not edit.\n\npub mod public;\npub mod admin;\n"
        );
    }

    #[test]
    fn rejects_invalid_mount_names() {
        let fixture = Fixture::new("invalid_mount");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");

        let error = discover_mount(Mount::new(
            "public-routes",
            &fixture.pages,
            "/",
            "crate::pages",
        ))
        .unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::InvalidMountName { mount_name } if mount_name == "public-routes"
        ));
    }

    #[test]
    fn rejects_index_as_intermediate_segment_and_show_pages() {
        let fixture = Fixture::new("nested_index_segment");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/index/details.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::ReservedPageSegment { segment, .. } if segment == "index"
        ));

        let fixture = Fixture::new("show_page");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/show.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::ReservedPageSegment { segment, .. } if segment == "show"
        ));
    }

    #[test]
    fn rejects_route_files_that_are_also_namespace_parents() {
        let fixture = Fixture::new("namespace_parent");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
        fixture.write("orders/new.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::ReservedPageSegment { segment, .. } if segment == "orders"
        ));
    }

    #[test]
    fn rejects_mutation_action_names_as_path_segments() {
        let fixture = Fixture::new("action_path_segments");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/create/confirmation.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::ReservedPageSegment { segment, .. } if segment == "create"
        ));
    }

    #[test]
    fn rejects_route_modules_without_expected_handler() {
        let fixture = Fixture::new("missing_handler");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write_source(
            "orders/index.rs",
            "pub(crate) async fn index() {}\npub(crate) async fn export_csv() {}\npub(crate) async fn items_json() {}\n",
        );

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::MissingHandler {
                source_file,
                handler_name,
            } if source_file.ends_with("orders/index.rs") && handler_name == "handler"
        ));
    }

    #[test]
    fn validates_custom_handler_names() {
        let fixture = Fixture::new("custom_handler");
        fixture.write_source("index.rs", "pub(crate) async fn route() {}\n");
        fixture.write_source("not_found_.rs", "pub(crate) async fn route() {}\n");
        fixture.write_source("orders/index.rs", "pub async fn route() {}\n");

        let routes = discover_mount(fixture.mount().with_handler_name("route"))
            .unwrap()
            .routes;

        assert_eq!(routes[1].handler_path, "crate::pages::orders::index::route");
    }

    #[test]
    fn supports_route_action_handler_names() {
        let fixture = Fixture::new("route_action_handlers");
        fixture.write_source("index.rs", "pub(crate) async fn index() {}\n");
        fixture.write_source("not_found_.rs", "pub(crate) async fn not_found() {}\n");
        fixture.write_source("orders/index.rs", "pub(crate) async fn index() {}\n");
        fixture.write_source("orders/new.rs", "pub(crate) async fn new() {}\n");
        fixture.write_source("orders/create.rs", "pub(crate) async fn create() {}\n");
        fixture.write_source(
            "orders/order_id_/index.rs",
            "pub(crate) async fn index() {}\n",
        );
        fixture.write_source(
            "orders/order_id_/edit.rs",
            "pub(crate) async fn edit() {}\n",
        );
        fixture.write_source(
            "orders/order_id_/update.rs",
            "pub(crate) use super::index::action as update;\n",
        );
        fixture.write_source(
            "orders/order_id_/delete.rs",
            "pub(crate) async fn delete() {}\n",
        );
        fixture.write_source(
            "orders/export.rs",
            "pub(crate) use super::index::export_csv;\n",
        );
        fixture.write_source(
            "orders/items.rs",
            "pub(crate) use super::index::items_json;\n",
        );

        let routes = discover_mount(fixture.mount().with_route_action_handler_names())
            .unwrap()
            .routes;
        let handlers = routes
            .iter()
            .map(|route| format!("{} {} {}", route.method, route.path, route.handler_path))
            .collect::<Vec<_>>();

        assert_eq!(
            handlers,
            [
                "GET / crate::pages::index::index",
                "GET /orders crate::pages::orders::index::index",
                "POST /orders crate::pages::orders::create::create",
                "GET /orders/export.csv crate::pages::orders::export::export_csv",
                "GET /orders/items.json crate::pages::orders::items::items_json",
                "GET /orders/new crate::pages::orders::new::new",
                "GET /orders/{order_id} crate::pages::orders::order_id_::index::index",
                "POST /orders/{order_id} crate::pages::orders::order_id_::update::update",
                "POST /orders/{order_id}/delete crate::pages::orders::order_id_::delete::delete",
                "GET /orders/{order_id}/edit crate::pages::orders::order_id_::edit::edit",
                "GET /not_found crate::pages::not_found_::not_found",
            ]
        );
    }

    #[test]
    fn validates_multiline_handler_declarations() {
        let fixture = Fixture::new("multiline_handler");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write_source(
            "orders/index.rs",
            r#"
#[allow(dead_code)]
pub(crate) async fn handler(
    state: (),
) {
}
"#,
        );

        let routes = discover_mount(fixture.mount()).unwrap().routes;

        assert_eq!(
            routes[1].handler_path,
            "crate::pages::orders::index::handler"
        );
    }

    #[test]
    fn ignores_public_struct_fields_when_validating_handlers() {
        let fixture = Fixture::new("public_struct_fields_before_handler");
        fixture.write_source("index.rs", "pub(crate) async fn index() {}\n");
        fixture.write_source(
            "orders/new.rs",
            r#"
pub(super) struct OrderForm {
    pub(super) name: String,
}

pub(crate) async fn new(
    state: (),
) {
}
"#,
        );

        let routes = discover_mount(fixture.mount().with_route_action_handler_names())
            .unwrap()
            .routes;

        assert!(routes.iter().any(|route| {
            route.path == "/orders/new" && route.handler_path == "crate::pages::orders::new::new"
        }));
    }

    #[test]
    fn rejects_duplicate_dynamic_route_patterns_for_same_method() {
        let fixture = Fixture::new("duplicate_patterns");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/id_/index.rs");
        fixture.write("orders/slug_/index.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::DuplicateRoute {
                method: HttpMethod::Get,
                path,
                ..
            } if path == "/orders/{_}"
        ));
    }

    #[test]
    fn allows_get_and_post_on_the_same_path() {
        let fixture = Fixture::new("same_path_different_methods");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/create.rs");

        let routes = discover_mount(fixture.mount()).unwrap().routes;
        let methods = routes
            .iter()
            .filter(|route| route.path == "/orders")
            .map(|route| route.method)
            .collect::<Vec<_>>();

        assert_eq!(methods, [HttpMethod::Get, HttpMethod::Post]);
    }

    #[test]
    fn rejects_duplicate_route_params() {
        let fixture = Fixture::new("duplicate_params");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("users/user_/repos/user_/index.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::DuplicateRouteParam { param, .. } if param == "user"
        ));
    }

    #[test]
    fn ignores_shared_dirs_and_mod_rs() {
        let fixture = Fixture::new("ignored_files");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("shared/header.rs");
        fixture.write("orders/shared/form.rs");
        fixture.write("orders/mod.rs");

        let routes = discover_mount(fixture.mount()).unwrap().routes;
        let paths = routes
            .iter()
            .map(|route| route.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(paths, ["/", "/not_found"]);
    }

    #[test]
    fn ignores_configured_path_prefixes() {
        let fixture = Fixture::new("ignored_prefixes");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("cart/index.rs");
        fixture.write("products/index.rs");
        fixture.write("products/id_/index.rs");
        fixture.write("content/articles/index.rs");

        let routes = discover_mount(
            fixture
                .mount()
                .with_ignored_path_prefixes(["products", "content"]),
        )
        .unwrap()
        .routes;
        let paths = routes
            .iter()
            .map(|route| route.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(paths, ["/", "/cart", "/not_found"]);
    }

    #[test]
    fn generates_not_found_fallback_without_not_found_file() {
        let fixture = Fixture::new("missing_not_found");
        fixture.write("index.rs");

        let mount_routes = discover_mount(fixture.mount()).unwrap();
        let generated = generate_mount_module(&mount_routes);
        let route_paths = mount_routes
            .routes
            .iter()
            .map(|route| route.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(route_paths, ["/"]);
        assert!(generated.contains("pub const fn home() -> &'static str"));
        assert!(!generated.contains("path: \"/not_found\""));
        assert!(!generated.contains(".route(\"/not_found\""));
    }

    #[test]
    fn generates_terminal_catch_all_routes() {
        let fixture = Fixture::new("catch_all");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("docs/all_.rs");

        let routes = discover_mount(fixture.mount()).unwrap().routes;
        let docs = routes
            .iter()
            .find(|route| route.name == "docs/all_")
            .expect("catch-all route");

        assert_eq!(docs.path, "/docs/{*all}");
        assert_eq!(
            docs.params,
            vec![RouteParam {
                name: "all".to_string(),
                type_name: "String".to_string(),
            }]
        );

        let generated = generate_mount_module(&MountRoutes {
            mount: fixture.mount(),
            routes,
        });
        assert!(generated.contains(r#"path: "/docs/{*all}""#));
        assert!(generated.contains("pub fn docs_all_(all: impl std::fmt::Display) -> String"));
        assert!(generated.contains("percent_encode_path(&all.to_string())"));
    }

    struct Fixture {
        root: PathBuf,
        pages: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("proute-{name}-{nonce}"));
            let pages = root.join("pages");
            fs::create_dir_all(&pages).unwrap();
            Self { root, pages }
        }

        fn write(&self, relative: &str) {
            let path = self.pages.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "pub(crate) async fn handler() {}\n").unwrap();
        }

        fn write_source(&self, relative: &str, source: &str) {
            let path = self.pages.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }

        fn mount(&self) -> Mount {
            Mount::new("public", &self.pages, "/", "crate::pages")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_fixture_file(root: &StdPath, relative: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "pub(crate) async fn handler() {}\n").unwrap();
    }

    fn generated_router_compile_wrapper(generated: &str) -> String {
        format!(
            r#"
mod axum {{
    use std::marker::PhantomData;

    pub struct Router<T>(PhantomData<T>);

    impl<T> Router<T> {{
        pub fn new() -> Self {{
            Self(PhantomData)
        }}

        pub fn route(self, _path: &str, _method_router: routing::MethodRouter<T>) -> Self {{
            self
        }}
    }}

    pub mod routing {{
        use std::marker::PhantomData;

        pub struct MethodRouter<T>(PhantomData<T>);

        impl<T> MethodRouter<T> {{
            pub fn post<H>(self, _handler: H) -> Self {{
                self
            }}

            pub fn delete<H>(self, _handler: H) -> Self {{
                self
            }}
        }}

        pub fn get<T, H>(_handler: H) -> MethodRouter<T> {{
            MethodRouter(PhantomData)
        }}

        pub fn post<T, H>(_handler: H) -> MethodRouter<T> {{
            MethodRouter(PhantomData)
        }}

        pub fn delete<T, H>(_handler: H) -> MethodRouter<T> {{
            MethodRouter(PhantomData)
        }}
    }}
}}

mod app {{
    pub struct AppState;
}}

mod pages {{
    pub mod index {{
        pub async fn handler() {{}}
    }}

    pub mod not_found_ {{
        pub async fn handler() {{}}
    }}

    pub mod orders {{
        pub mod index {{
            pub async fn handler() {{}}
        }}

        pub mod create {{
            pub async fn handler() {{}}
        }}

        pub mod order_id_ {{
            pub mod index {{
                pub async fn handler() {{}}
            }}

            pub mod update {{
                pub async fn handler() {{}}
            }}

            pub mod delete {{
                pub async fn handler() {{}}
            }}
        }}
    }}
}}

{generated}
"#
        )
    }

    fn generated_typed_helper_compile_wrapper(generated: &str) -> String {
        format!(
            r#"
mod proute {{
    pub trait ToParam {{
        fn to_param(&self) -> String;
    }}

    pub trait IntoParam<T> {{
        fn into_param(self) -> String;
    }}

    impl ToParam for i64 {{
        fn to_param(&self) -> String {{
            self.to_string()
        }}
    }}

    impl ToParam for str {{
        fn to_param(&self) -> String {{
            self.to_string()
        }}
    }}

    impl ToParam for String {{
        fn to_param(&self) -> String {{
            self.clone()
        }}
    }}

    impl<T> IntoParam<T> for T
    where
        T: ToParam,
    {{
        fn into_param(self) -> String {{
            self.to_param()
        }}
    }}

    impl<T> IntoParam<T> for &T
    where
        T: ToParam,
    {{
        fn into_param(self) -> String {{
            self.to_param()
        }}
    }}

    impl IntoParam<String> for &str {{
        fn into_param(self) -> String {{
            self.to_string()
        }}
    }}

    pub struct FriendlyId<T> {{
        pub id: T,
    }}

    impl<T: ToParam> ToParam for FriendlyId<T> {{
        fn to_param(&self) -> String {{
            self.id.to_param()
        }}
    }}

    impl<T> IntoParam<FriendlyId<T>> for T
    where
        T: ToParam,
    {{
        fn into_param(self) -> String {{
            self.to_param()
        }}
    }}
}}

mod pages {{
    pub mod index {{
        pub async fn handler() {{}}
    }}

    pub mod orders {{
        pub mod order_id_ {{
            pub mod index {{
                pub(crate) struct RouteParams {{
                    pub(crate) order_id: i64,
                }}

                pub async fn handler() {{}}
            }}
        }}
    }}

    pub mod products {{
        pub mod product_type_ {{
            pub mod index {{
                pub(crate) struct RouteParams {{
                    pub(crate) product_type: String,
                }}

                pub async fn handler() {{}}
            }}
        }}
    }}

    pub mod pages {{
        pub mod id_ {{
            pub mod index {{
                pub(crate) struct RouteParams {{
                    pub(crate) id: crate::proute::FriendlyId<i64>,
                }}

                pub async fn handler() {{}}
            }}
        }}
    }}
}}

{generated}

fn main() {{
    assert_eq!(orders_order_id_(123), "/orders/123");
    assert_eq!(prefixed_orders_order_id_("fr", 123), "/fr/orders/123");
    assert_eq!(products_product_type_("leagues"), "/products/leagues");
    assert_eq!(pages_id_(123), "/pages/123");
    assert_eq!(
        localized_orders_order_id_("en", "en", 123),
        "/orders/123"
    );
    assert_eq!(
        localized_orders_order_id_("fr", "en", 123),
        "/fr/orders/123"
    );
}}
"#
        )
    }
}
