use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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
}

impl Route {
    pub fn pattern_path(&self) -> String {
        pattern_path(&self.path)
    }
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
    UnsupportedCatchAll {
        source_file: PathBuf,
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
}

impl fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoverError::PagesDirectoryUnreadable { path } => {
                write!(f, "could not read pages directory {}", path.display())
            }
            DiscoverError::UnsupportedCatchAll { source_file } => write!(
                f,
                "unsupported catch-all page {}: all_.rs is reserved but not generated yet",
                source_file.display()
            ),
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

pub fn write_mount_file(output_root: &Path, mount: Mount) -> Result<GeneratedFile, WriteError> {
    let mount_routes = discover_mount(mount).map_err(WriteError::Discover)?;
    let generated = generate_mount_file(&mount_routes);
    write_generated_file(output_root, &generated)?;

    Ok(generated)
}

pub fn write_mount_files(
    output_root: &Path,
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

fn write_generated_file(output_root: &Path, generated: &GeneratedFile) -> Result<(), WriteError> {
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
        path: PathBuf::from("routes").join(format!("{}.rs", mount_routes.mount.name)),
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
        path: PathBuf::from("routes").join("mod.rs"),
        contents: format!("//// Generated. Do not edit.\n\n{modules}\n"),
    }
}

pub fn generate_mount_module(mount_routes: &MountRoutes) -> String {
    let mut sections = Vec::new();
    sections.push(generated_header(mount_routes));
    sections.push(route_spec_type());
    sections.push(route_enum(mount_routes));
    sections.push(parsed_request_type(mount_routes));
    sections.push(route_table(mount_routes));
    sections.push(router_functions(mount_routes));
    sections.push(parse_request(mount_routes));
    sections.push(parse_localized_request(mount_routes));
    sections.push(route_to_path(mount_routes));
    sections.push(route_to_prefixed_path(mount_routes));
    sections.push(route_to_localized_path(mount_routes));
    sections.push(route_to_url());
    sections.push(route_to_localized_url(mount_routes));
    sections.push(path_helpers(mount_routes));
    sections.push(path_segments_function());
    sections.push(percent_encode_function());
    sections.push(percent_decode_function());
    sections.push(trim_trailing_slash());

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

fn route_enum(mount_routes: &MountRoutes) -> String {
    let display_routes = routes_with_synthetic_not_found(mount_routes);
    let variants = display_routes
        .iter()
        .map(route_variant)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "#[derive(Clone, Debug, Eq, PartialEq)]\npub enum Route {{\n{}\n}}\n",
        indent_lines(&variants, 4)
    )
}

fn parsed_request_type(mount_routes: &MountRoutes) -> String {
    let Some(language_param) = mount_routes.mount.language_param.as_deref() else {
        return String::new();
    };

    format!(
        "#[derive(Clone, Debug, Eq, PartialEq)]\npub struct ParsedRequest {{\n    pub route: Route,\n    pub {language_param}: Option<String>,\n}}\n"
    )
}

fn route_variant(route: &Route) -> String {
    if route.params.is_empty() {
        format!("{},", route.name)
    } else {
        let fields = route
            .params
            .iter()
            .map(|param| format!("{}: {}", param.name, param.type_name))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} {{ {} }},", route.name, fields)
    }
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

fn parse_request(mount_routes: &MountRoutes) -> String {
    let cases = mount_routes
        .routes
        .iter()
        .map(parse_request_case)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"pub fn parse_request(method: &str, raw_path: &str) -> Route {{
    let path = raw_path.split(['?', '#']).next().unwrap_or(raw_path);
    let segments = path_segments(path);
    parse_segments(method, segments.as_slice())
}}

fn parse_segments(method: &str, segments: &[&str]) -> Route {{
    match (method, segments) {{
{}
        _ => Route::NotFound,
    }}
}}
"#,
        indent_lines(&cases, 8)
    )
}

fn parse_localized_request(mount_routes: &MountRoutes) -> String {
    let Some(language_param) = mount_routes.mount.language_param.as_deref() else {
        return String::new();
    };

    format!(
        r#"pub fn parse_localized_request(method: &str, raw_path: &str) -> ParsedRequest {{
    let path = raw_path.split(['?', '#']).next().unwrap_or(raw_path);
    let segments = path_segments(path);
    let canonical = parse_segments(method, segments.as_slice());

    if canonical != Route::NotFound {{
        return ParsedRequest {{
            route: canonical,
            {language_param}: None,
        }};
    }}

    match segments.as_slice() {{
        [{language_param}, rest @ ..] => {{
            let Some({language_param}) = percent_decode({language_param}) else {{
                return ParsedRequest {{
                    route: Route::NotFound,
                    {language_param}: None,
                }};
            }};
            let route = parse_segments(method, rest);
            ParsedRequest {{
                route,
                {language_param}: Some({language_param}),
            }}
        }}
        _ => ParsedRequest {{
            route: parse_segments(method, segments.as_slice()),
            {language_param}: None,
        }},
    }}
}}
"#
    )
}

fn parse_request_case(route: &Route) -> String {
    let pattern = parse_segment_pattern(route);
    if route.params.is_empty() {
        format!(
            "({:?}, {pattern}) => Route::{},",
            route.method.to_string(),
            route.name
        )
    } else {
        let decoders = parse_dynamic_decoders(route);
        format!(
            "({:?}, {pattern}) => {{\n{}\n        }},",
            route.method.to_string(),
            indent_lines(&decoders, 12)
        )
    }
}

fn parse_segment_pattern(route: &Route) -> String {
    if route.segments.is_empty() {
        return "[]".to_string();
    }

    let segments = route
        .segments
        .iter()
        .map(|segment| match segment {
            RouteSegment::Static(value) => format!("{value:?}"),
            RouteSegment::Dynamic(name) => name.as_str().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{segments}]")
}

fn parse_dynamic_decoders(route: &Route) -> String {
    let mut lines = Vec::new();
    for param in &route.params {
        lines.push(format!(
            "let Some({}) = percent_decode({}) else {{ return Route::NotFound; }};",
            param.name, param.name
        ));
    }

    let fields = route
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, param.name))
        .collect::<Vec<_>>()
        .join(", ");

    lines.push(format!("Route::{} {{ {} }}", route.name, fields));
    lines.join("\n")
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

fn route_to_path(mount_routes: &MountRoutes) -> String {
    let display_routes = routes_with_synthetic_not_found(mount_routes);
    let cases = display_routes
        .iter()
        .map(route_to_path_case)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "pub fn route_to_path(route: &Route) -> String {{\n    match route {{\n{}\n    }}\n}}\n",
        indent_lines(&cases, 8)
    )
}

fn route_to_prefixed_path(mount_routes: &MountRoutes) -> String {
    let Some(language_param) = mount_routes.mount.language_param.as_deref() else {
        return String::new();
    };

    let display_routes = routes_with_synthetic_not_found(mount_routes);
    let cases = display_routes
        .iter()
        .map(|route| route_to_prefixed_path_case(&mount_routes.mount, route))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "pub fn route_to_prefixed_path(route: &Route, {language_param}: &str) -> String {{\n    match route {{\n{}\n    }}\n}}\n",
        indent_lines(&cases, 8)
    )
}

fn route_to_prefixed_path_case(mount: &Mount, route: &Route) -> String {
    let pattern = route_pattern(route);
    let Some(prefixed_path) = prefixed_route_path(mount, &route.path) else {
        return format!("{pattern} => route_to_path(route),");
    };

    let expression = path_expression_from_template(&prefixed_path);
    format!("{pattern} => {expression},")
}

fn route_to_localized_path(mount_routes: &MountRoutes) -> String {
    let Some(language_param) = mount_routes.mount.language_param.as_deref() else {
        return String::new();
    };

    format!(
        r#"pub fn route_to_localized_path(route: &Route, {language_param}: &str, primary_lang: &str) -> String {{
    if {language_param} == primary_lang {{
        route_to_path(route)
    }} else {{
        route_to_prefixed_path(route, {language_param})
    }}
}}
"#
    )
}

fn route_to_path_case(route: &Route) -> String {
    let pattern = route_pattern(route);
    if route.params.is_empty() {
        format!("{pattern} => {:?}.to_string(),", route.path)
    } else {
        format!(
            "{pattern} => {}({}),",
            route.helper_name,
            helper_args(route)
        )
    }
}

fn route_pattern(route: &Route) -> String {
    if route.params.is_empty() {
        format!("Route::{}", route.name)
    } else {
        let fields = route
            .params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("Route::{} {{ {} }}", route.name, fields)
    }
}

fn helper_args(route: &Route) -> String {
    route
        .params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn route_to_url() -> String {
    r#"pub fn route_to_url(route: &Route, origin: &str) -> String {
    format!("{}{}", trim_trailing_slash(origin), route_to_path(route))
}
"#
    .to_string()
}

fn routes_with_synthetic_not_found(mount_routes: &MountRoutes) -> Vec<Route> {
    let mut routes = mount_routes.routes.clone();
    if !routes.iter().any(|route| route.kind == RouteKind::NotFound) {
        routes.push(synthetic_not_found_route(&mount_routes.mount));
    }
    routes
}

fn route_to_localized_url(mount_routes: &MountRoutes) -> String {
    let Some(language_param) = mount_routes.mount.language_param.as_deref() else {
        return String::new();
    };

    format!(
        r#"pub fn route_to_localized_url(route: &Route, origin: &str, {language_param}: &str, primary_lang: &str) -> String {{
    format!("{{}}{{}}", trim_trailing_slash(origin), route_to_localized_path(route, {language_param}, primary_lang))
}}
"#
    )
}

fn path_helpers(mount_routes: &MountRoutes) -> String {
    mount_routes
        .routes
        .iter()
        .map(path_helper)
        .collect::<Vec<_>>()
        .join("\n")
}

fn path_segments_function() -> String {
    r#"fn path_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}
"#
    .to_string()
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

fn is_unreserved(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
    )
}
"#
    .to_string()
}

fn percent_decode_function() -> String {
    r#"fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push(hex_value(high)? * 16 + hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
"#
    .to_string()
}

fn path_helper(route: &Route) -> String {
    if route.params.is_empty() {
        format!(
            "pub fn {}() -> &'static str {{\n    {:?}\n}}\n",
            route.helper_name, route.path
        )
    } else {
        let args = route
            .params
            .iter()
            .map(|param| format!("{}: impl std::fmt::Display", param.name))
            .collect::<Vec<_>>()
            .join(", ");
        let expression = path_expression(route);

        format!(
            "pub fn {}({}) -> String {{\n    {}\n}}\n",
            route.helper_name, args, expression
        )
    }
}

fn path_expression(route: &Route) -> String {
    path_expression_from_template(&route.path)
}

fn path_expression_from_template(path: &str) -> String {
    let mut params = Vec::new();
    let template = path
        .split('/')
        .map(|segment| {
            if let Some(param) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                params.push(param.to_string());
                "{}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    let encoded_params = params
        .iter()
        .map(|param| format!("percent_encode(&{param}.to_string())"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("format!({template:?}, {encoded_params})")
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

fn trim_trailing_slash() -> String {
    r#"fn trim_trailing_slash(value: &str) -> &str {
    value.strip_suffix('/').unwrap_or(value)
}
"#
    .to_string()
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

fn walk_pages(root: &Path) -> Result<Vec<PathBuf>, DiscoverError> {
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

fn should_ignore_file(mount: &Mount, file: &Path) -> bool {
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

fn route_from_file(mount: &Mount, source_file: &Path) -> Result<Route, DiscoverError> {
    let raw_segments = raw_segments(mount, source_file)?;

    if raw_segments.iter().any(|segment| segment == "all_") {
        return Err(DiscoverError::UnsupportedCatchAll {
            source_file: source_file.to_path_buf(),
        });
    }

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
    })
}

fn raw_segments(mount: &Mount, source_file: &Path) -> Result<Vec<String>, DiscoverError> {
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
    if let Some(param) = segment.strip_suffix('_') {
        RouteSegment::Dynamic(param.to_string())
    } else {
        RouteSegment::Static(segment.to_string())
    }
}

fn route_kind(segments: &[RouteSegment]) -> RouteKind {
    if segments.is_empty() {
        return RouteKind::Home;
    }

    if segments
        .iter()
        .any(|segment| matches!(segment, RouteSegment::Dynamic(_)))
    {
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
            RouteSegment::Dynamic(name) => Some(RouteParam {
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

fn not_found_route(mount: &Mount, source_file: &Path, module_path: String) -> Route {
    let segments = vec![RouteSegment::Static("not_found".to_string())];
    let handler_name = handler_name_for(mount, &["not_found_".to_string()]);
    let handler_path = format!("{module_path}::{handler_name}");

    Route {
        kind: RouteKind::NotFound,
        endpoint: Endpoint::Page,
        method: HttpMethod::Get,
        name: "NotFound".to_string(),
        helper_name: "not_found".to_string(),
        path: route_path(&mount.route_root, &segments),
        segments,
        params: Vec::new(),
        source_file: source_file.to_path_buf(),
        module_path,
        handler_name,
        handler_path,
    }
}

fn synthetic_not_found_route(mount: &Mount) -> Route {
    let source_file = mount.pages.join("not_found_.rs");
    let segments = vec![RouteSegment::Static("not_found".to_string())];

    Route {
        kind: RouteKind::NotFound,
        endpoint: Endpoint::Page,
        method: HttpMethod::Get,
        name: "NotFound".to_string(),
        helper_name: "not_found".to_string(),
        path: route_path(&mount.route_root, &segments),
        segments,
        params: Vec::new(),
        source_file,
        module_path: String::new(),
        handler_name: String::new(),
        handler_path: String::new(),
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
    let words = raw_segments
        .iter()
        .filter(|segment| segment.as_str() != "index")
        .flat_map(|segment| segment.trim_end_matches('_').split('_'))
        .filter(|word| !word.is_empty())
        .map(capitalize)
        .collect::<String>();

    if words.is_empty() {
        "Home".to_string()
    } else {
        words
    }
}

fn helper_name(raw_segments: &[String]) -> String {
    let helper = raw_segments
        .iter()
        .filter(|segment| segment.as_str() != "index")
        .map(|segment| segment.trim_end_matches('_'))
        .collect::<Vec<_>>()
        .join("_");

    if helper.is_empty() {
        "home".to_string()
    } else {
        helper
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn module_path(mount: &Mount, source_file: &Path) -> Result<String, DiscoverError> {
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

fn validate_raw_segments(source_file: &Path, raw_segments: &[String]) -> Result<(), DiscoverError> {
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
    source_file: &Path,
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
        segment if segment.ends_with('_') && is_last && segment != "not_found_" => true,
        "create" | "update" | "delete" => !is_last,
        _ => false,
    }
}

fn validate_params(source_file: &Path, params: &[RouteParam]) -> Result<(), DiscoverError> {
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
            || declaration.starts_with(&format!("pub(crate) use "))
                && declaration.ends_with(&format!("::{handler_name};"))
            || declaration.starts_with(&format!("pub use "))
                && declaration.ends_with(&format!("::{handler_name};"))
    })
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

fn relative_parts(root: &Path, file: &Path) -> Result<Vec<String>, ()> {
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
                "GET / Home crate::pages::index",
                "GET /orders Orders crate::pages::orders::index",
                "POST /orders OrdersCreate crate::pages::orders::create",
                "GET /orders/export.csv OrdersExport crate::pages::orders::export",
                "GET /orders/items.json OrdersItems crate::pages::orders::items",
                "GET /orders/new OrdersNew crate::pages::orders::new",
                "GET /orders/{order_id} OrdersOrderId crate::pages::orders::order_id_::index",
                "POST /orders/{order_id} OrdersOrderIdUpdate crate::pages::orders::order_id_::update",
                "POST /orders/{order_id}/delete OrdersOrderIdDelete crate::pages::orders::order_id_::delete",
                "GET /orders/{order_id}/edit OrdersOrderIdEdit crate::pages::orders::order_id_::edit",
                "GET /not_found NotFound crate::pages::not_found_",
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
    fn generated_file_targets_routes_directory() {
        let fixture = Fixture::new("generated_path");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/order_id_/index.rs");

        let mount_routes = discover_mount(fixture.mount().with_language_param("lang")).unwrap();
        let generated = generate_mount_file(&mount_routes);

        assert_eq!(generated.path, PathBuf::from("routes/public.rs"));
        assert!(generated.contents.contains("//// mount: public"));
        assert!(generated.contents.contains("pub enum Route"));
        assert!(generated.contents.contains("pub const ROUTES"));
        assert!(
            generated
                .contents
                .contains("pub fn orders_order_id(order_id: impl std::fmt::Display) -> String")
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
                .contains("pub fn route_to_prefixed_path(route: &Route, lang: &str) -> String")
        );
        assert!(
            generated
                .contents
                .contains("pub fn route_to_localized_path(route: &Route, lang: &str, primary_lang: &str) -> String")
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
    fn generated_parser_handles_methods_and_percent_decoded_params() {
        let fixture = Fixture::new("generated_parser");
        fixture.write("index.rs");
        fixture.write("orders/index.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_/index.rs");

        let mount_routes = discover_mount(fixture.mount().with_language_param("lang")).unwrap();
        let source_path = fixture.root.join("generated_parser.rs");
        fs::write(
            &source_path,
            generated_parser_runtime_wrapper(&generate_mount_module(&mount_routes)),
        )
        .unwrap();

        let binary_path = fixture.root.join("generated_parser");
        let compile = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .unwrap();

        assert!(
            compile.status.success(),
            "generated parser binary did not compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let run = std::process::Command::new(binary_path).output().unwrap();

        assert!(
            run.status.success(),
            "generated parser binary failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
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
    fn write_mount_file_writes_under_routes_directory() {
        let fixture = Fixture::new("write_mount_file");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");

        let output_root = fixture.root.join("generated");
        let generated = write_mount_file(&output_root, fixture.mount()).unwrap();
        let output_path = output_root.join("routes/public.rs");

        assert_eq!(generated.path, PathBuf::from("routes/public.rs"));
        assert!(output_path.exists());
        assert!(
            fs::read_to_string(output_path)
                .unwrap()
                .contains("pub enum Route")
        );
    }

    #[test]
    fn write_mount_files_writes_mount_modules_and_routes_mod() {
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
                PathBuf::from("routes/public.rs"),
                PathBuf::from("routes/admin.rs"),
                PathBuf::from("routes/mod.rs"),
            ]
        );
        assert!(output_root.join("routes/public.rs").exists());
        assert!(output_root.join("routes/admin.rs").exists());
        assert_eq!(
            fs::read_to_string(output_root.join("routes/mod.rs")).unwrap(),
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
            "pub(crate) async fn update() {}\n",
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
        assert!(generated.contains("pub enum Route {\n    Home,\n    NotFound,\n}"));
        assert!(generated.contains("_ => Route::NotFound,"));
        assert!(generated.contains("Route::NotFound => \"/not_found\".to_string(),"));
        assert!(!generated.contains("path: \"/not_found\""));
        assert!(!generated.contains(".route(\"/not_found\""));
    }

    #[test]
    fn rejects_reserved_catch_all() {
        let fixture = Fixture::new("catch_all");
        fixture.write("index.rs");
        fixture.write("not_found_.rs");
        fixture.write("docs/all_.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(error, DiscoverError::UnsupportedCatchAll { .. }));
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

    fn write_fixture_file(root: &Path, relative: &str) {
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

    fn generated_parser_runtime_wrapper(generated: &str) -> String {
        format!(
            r#"
{generated}

fn main() {{
    assert_eq!(parse_request("GET", "/"), Route::Home);
    assert_eq!(parse_request("GET", "/orders"), Route::Orders);
    assert_eq!(parse_request("POST", "/orders"), Route::OrdersCreate);
    assert_eq!(
        parse_request("GET", "/orders/a%2Fb?tab=details"),
        Route::OrdersOrderId {{
            order_id: "a/b".to_string(),
        }}
    );
    assert_eq!(
        route_to_path(&Route::OrdersOrderId {{
            order_id: "a/b".to_string(),
        }}),
        "/orders/a%2Fb".to_string()
    );
    assert_eq!(parse_request("DELETE", "/orders/a%2Fb"), Route::NotFound);
    assert_eq!(parse_request("GET", "/orders/%GG"), Route::NotFound);
    assert_eq!(route_to_path(&Route::NotFound), "/not_found".to_string());
    assert_eq!(
        route_to_prefixed_path(&Route::NotFound, "fr"),
        "/fr/not_found".to_string()
    );
    assert_eq!(
        parse_localized_request("GET", "/orders"),
        ParsedRequest {{
            route: Route::Orders,
            lang: None,
        }}
    );
    assert_eq!(
        parse_localized_request("GET", "/fr/orders/a%2Fb?tab=details"),
        ParsedRequest {{
            route: Route::OrdersOrderId {{
                order_id: "a/b".to_string(),
            }},
            lang: Some("fr".to_string()),
        }}
    );
    assert_eq!(
        parse_localized_request("POST", "/fr/orders"),
        ParsedRequest {{
            route: Route::OrdersCreate,
            lang: Some("fr".to_string()),
        }}
    );
    assert_eq!(
        parse_localized_request("GET", "/%GG/orders"),
        ParsedRequest {{
            route: Route::NotFound,
            lang: None,
        }}
    );
}}
"#
        )
    }
}
