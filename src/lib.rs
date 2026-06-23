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
    pub router_state_type: Option<String>,
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
            router_state_type: None,
        }
    }

    pub fn with_language_param(mut self, language_param: impl Into<String>) -> Self {
        self.language_param = Some(language_param.into());
        self
    }

    pub fn with_handler_name(mut self, handler_name: impl Into<String>) -> Self {
        self.handler_name = handler_name.into();
        self
    }

    pub fn with_router_state_type(mut self, router_state_type: impl Into<String>) -> Self {
        self.router_state_type = Some(router_state_type.into());
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
    MissingNotFound {
        mount_name: String,
        pages: PathBuf,
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
}

impl fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoverError::PagesDirectoryUnreadable { path } => {
                write!(f, "could not read pages directory {}", path.display())
            }
            DiscoverError::MissingNotFound { mount_name, pages } => write!(
                f,
                "mount {mount_name:?} at {} is missing not_found_.rs",
                pages.display()
            ),
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
    if !is_valid_label(&mount.handler_name) {
        return Err(DiscoverError::InvalidRouteName {
            source_file: mount.pages.clone(),
            name: mount.handler_name.clone(),
        });
    }

    let files = walk_pages(&mount.pages)?;
    let mut routes = Vec::new();

    for file in files {
        if should_ignore_file(&mount.pages, &file) {
            continue;
        }

        routes.push(route_from_file(&mount, &file)?);
    }

    require_not_found(&mount, &routes)?;
    reject_duplicate_routes(&routes)?;
    reject_duplicate_names(&routes)?;
    reject_duplicate_helpers(&routes)?;
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
    sections.push(route_table(mount_routes));
    sections.push(router_functions(mount_routes));
    sections.push(route_to_path(mount_routes));
    sections.push(route_to_prefixed_path(mount_routes));
    sections.push(route_to_localized_path(mount_routes));
    sections.push(route_to_url());
    sections.push(route_to_localized_url(mount_routes));
    sections.push(path_helpers(mount_routes));
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
    let variants = mount_routes
        .routes
        .iter()
        .map(route_variant)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "#[derive(Clone, Debug, Eq, PartialEq)]\npub enum Route {{\n{}\n}}\n",
        indent_lines(&variants, 4)
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
    let cases = mount_routes
        .routes
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

    let cases = mount_routes
        .routes
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

    format!("format!({template:?}, {})", params.join(", "))
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

fn should_ignore_file(root: &Path, file: &Path) -> bool {
    if file.extension().is_none_or(|extension| extension != "rs") {
        return true;
    }

    if file.file_name().is_some_and(|name| name == "mod.rs") {
        return true;
    }

    relative_parts(root, file).is_ok_and(|parts| parts.iter().any(|part| part == "shared"))
}

fn route_from_file(mount: &Mount, source_file: &Path) -> Result<Route, DiscoverError> {
    let raw_segments = raw_segments(mount, source_file)?;

    if raw_segments.iter().any(|segment| segment == "all_") {
        return Err(DiscoverError::UnsupportedCatchAll {
            source_file: source_file.to_path_buf(),
        });
    }

    validate_raw_segments(source_file, &raw_segments)?;

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

    let handler_path = format!("{module_path}::{}", mount.handler_name);

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
        Endpoint::Action(Action::Create | Action::Update) => HttpMethod::Post,
        Endpoint::Action(Action::Delete) => HttpMethod::Delete,
    }
}

fn route_segments(raw_segments: &[String], endpoint: &Endpoint) -> Vec<RouteSegment> {
    let path_segments = match endpoint {
        Endpoint::Page => raw_segments,
        Endpoint::Action(_) => &raw_segments[..raw_segments.len() - 1],
    };

    path_segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| {
            if segment == "home_" && index == path_segments.len() - 1 {
                None
            } else {
                Some(route_segment(segment))
            }
        })
        .collect()
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
    let handler_path = format!("{module_path}::{}", mount.handler_name);

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
        handler_path,
    }
}

fn route_name(raw_segments: &[String]) -> String {
    let words = raw_segments
        .iter()
        .filter(|segment| segment.as_str() != "home_")
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
        .filter(|segment| segment.as_str() != "home_")
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
    for segment in raw_segments {
        if !is_valid_module_segment(segment) {
            return Err(DiscoverError::InvalidPageSegment {
                source_file: source_file.to_path_buf(),
                segment: segment.clone(),
            });
        }
    }

    Ok(())
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

fn require_not_found(mount: &Mount, routes: &[Route]) -> Result<(), DiscoverError> {
    if routes.iter().any(|route| route.kind == RouteKind::NotFound) {
        Ok(())
    } else {
        Err(DiscoverError::MissingNotFound {
            mount_name: mount.name.clone(),
            pages: mount.pages.clone(),
        })
    }
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
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
        fixture.write("orders/new.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_.rs");
        fixture.write("orders/order_id_/edit.rs");
        fixture.write("orders/order_id_/update.rs");
        fixture.write("orders/order_id_/delete.rs");
        fixture.write("orders/export.rs");
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
                "GET / Home crate::pages::home_",
                "GET /orders Orders crate::pages::orders",
                "POST /orders OrdersCreate crate::pages::orders::create",
                "GET /orders/export OrdersExport crate::pages::orders::export",
                "GET /orders/new OrdersNew crate::pages::orders::new",
                "GET /orders/{order_id} OrdersOrderId crate::pages::orders::order_id_",
                "POST /orders/{order_id} OrdersOrderIdUpdate crate::pages::orders::order_id_::update",
                "DELETE /orders/{order_id} OrdersOrderIdDelete crate::pages::orders::order_id_::delete",
                "GET /orders/{order_id}/edit OrdersOrderIdEdit crate::pages::orders::order_id_::edit",
                "GET /not_found NotFound crate::pages::not_found_",
            ]
        );
    }

    #[test]
    fn supports_mount_roots() {
        let fixture = Fixture::new("mount_roots");
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("games/id_.rs");

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
                "GET /admin crate::pages::admin::home_",
                "GET /admin/games/{id} crate::pages::admin::games::id_",
                "GET /admin/not_found crate::pages::admin::not_found_",
            ]
        );
    }

    #[test]
    fn generated_file_targets_routes_directory() {
        let fixture = Fixture::new("generated_path");
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/order_id_.rs");

        let mount_routes = discover_mount(fixture.mount()).unwrap();
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
                .contains("format!(\"/orders/{}\", order_id)")
        );
    }

    #[test]
    fn generated_routes_include_i18n_prefixes_when_mount_requests_them() {
        let fixture = Fixture::new("i18n_routes");
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
        fixture.write("orders/order_id_.rs");

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
        assert!(generated.contents.contains("format!(\"/{}/orders\", lang)"));
        assert!(
            generated
                .contents
                .contains("format!(\"/{}/orders/{}\", lang, order_id)")
        );
    }

    #[test]
    fn generated_module_compiles_as_rust() {
        let fixture = Fixture::new("generated_compiles");
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
        fixture.write("orders/order_id_.rs");
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
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_.rs");
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
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
        fixture.write("orders/create.rs");
        fixture.write("orders/order_id_.rs");
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
                .contains(".route(\"/orders\", axum::routing::get(crate::pages::orders::handler).post(crate::pages::orders::create::handler))")
        );
        assert!(
            generated
                .contains(".route(\"/orders/{order_id}\", axum::routing::get(crate::pages::orders::order_id_::handler).post(crate::pages::orders::order_id_::update::handler).delete(crate::pages::orders::order_id_::delete::handler))")
        );
        assert!(
            generated
                .contains(".route(\"/{lang}/orders\", axum::routing::get(crate::pages::orders::handler).post(crate::pages::orders::create::handler))")
        );
    }

    #[test]
    fn write_mount_file_writes_under_routes_directory() {
        let fixture = Fixture::new("write_mount_file");
        fixture.write("home_.rs");
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
        write_fixture_file(&public_pages, "home_.rs");
        write_fixture_file(&public_pages, "not_found_.rs");
        write_fixture_file(&admin_pages, "home_.rs");
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
        fixture.write("home_.rs");
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
    fn rejects_duplicate_dynamic_route_patterns_for_same_method() {
        let fixture = Fixture::new("duplicate_patterns");
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders/id_.rs");
        fixture.write("orders/slug_.rs");

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
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("orders.rs");
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
        fixture.write("home_.rs");
        fixture.write("not_found_.rs");
        fixture.write("users/user_/repos/user_.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(
            error,
            DiscoverError::DuplicateRouteParam { param, .. } if param == "user"
        ));
    }

    #[test]
    fn ignores_shared_dirs_and_mod_rs() {
        let fixture = Fixture::new("ignored_files");
        fixture.write("home_.rs");
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
    fn requires_not_found() {
        let fixture = Fixture::new("missing_not_found");
        fixture.write("home_.rs");

        let error = discover_mount(fixture.mount()).unwrap_err();

        assert!(matches!(error, DiscoverError::MissingNotFound { .. }));
    }

    #[test]
    fn rejects_reserved_catch_all() {
        let fixture = Fixture::new("catch_all");
        fixture.write("home_.rs");
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
            fs::write(path, "").unwrap();
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
        fs::write(path, "").unwrap();
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
    pub mod home_ {{
        pub async fn handler() {{}}
    }}

    pub mod not_found_ {{
        pub async fn handler() {{}}
    }}

    pub mod orders {{
        pub async fn handler() {{}}

        pub mod create {{
            pub async fn handler() {{}}
        }}

        pub mod order_id_ {{
            pub async fn handler() {{}}

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
}
