use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

extern crate edgerun_vfs;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

#[derive(Debug, Clone)]
struct Args {
    cmd: String,
    workspace: PathBuf,
    package: Option<String>,
    bin: Option<String>,
    bins: bool,
    lib_only: bool,
    example: Option<String>,
    examples: bool,
    features: BTreeSet<String>,
    all_features: bool,
    no_default_features: bool,
    release: bool,
    keep_ram: bool,
    fresh: bool,
    target: Option<String>,
    out: Option<PathBuf>,
    rest: Vec<String>,
}

#[derive(Debug, Clone)]
struct Workspace {
    packages: BTreeMap<String, Package>,
}

#[derive(Debug, Clone)]
struct Package {
    name: String,
    dir: String,
    edition: String,
    features: BTreeMap<String, Vec<String>>,
    deps: BTreeMap<String, Dep>,
    lib: Option<Target>,
    bins: Vec<Target>,
    examples: Vec<Target>,
    build_cfgs: Vec<String>,
}

#[derive(Debug, Clone)]
struct Target {
    name: String,
    path: String,
    kind: TargetKind,
    required_features: Vec<String>,
    crate_types: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TargetKind {
    Lib,
    ProcMacro,
    Bin,
    Example,
}

#[derive(Debug, Clone)]
struct Dep {
    package: String,
    rename: String,
    optional: bool,
    default_features: bool,
    features: Vec<String>,
}

#[derive(Debug, Clone)]
struct BuildNode {
    package: String,
    enabled_features: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct BuiltArtifact {
    path: PathBuf,
}

#[derive(Debug, Default, Clone)]
struct Manifest {
    package_name: Option<String>,
    edition: Option<String>,
    workspace_members: Vec<String>,
    features: BTreeMap<String, Vec<String>>,
    deps: BTreeMap<String, Dep>,
    lib_path: Option<String>,
    lib_proc_macro: bool,
    bins: Vec<Target>,
    examples: Vec<Target>,
}

#[derive(Debug, Clone)]
struct HostCfg {
    arch: String,
    os: String,
    pointer_width: String,
}

impl HostCfg {
    fn current() -> Self {
        Self {
            arch: std::env::consts::ARCH.to_string(),
            os: std::env::consts::OS.to_string(),
            pointer_width: if usize::BITS == 64 { "64" } else { "32" }.to_string(),
        }
    }

    fn from_target_triple(triple: &str) -> Self {
        let mut parts = triple.split('-');
        let arch = parts.next().unwrap_or_default();
        let os = if triple == "wasm32-unknown-unknown" {
            "unknown"
        } else {
            parts.nth(1).unwrap_or_default()
        };
        let pointer_width = if arch.contains("64") { "64" } else { "32" };
        Self {
            arch: arch.to_string(),
            os: os.to_string(),
            pointer_width: pointer_width.to_string(),
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let vfs = load_vfs(path_str(&args.workspace)?)?;
    let ws = index_workspace(&args, args.workspace.clone(), &vfs)?;
    match args.cmd.as_str() {
        "doctor" => doctor(&args),
        "graph" => {
            let root = select_package(&args, &ws)?;
            let nodes = resolve_closure(&ws, &root, &args)?;
            print_graph(&ws, &nodes);
            Ok(())
        }
        "clean" => clean_ram(),
        "check" | "build" | "test" | "run" => build_command(&args, &ws, &vfs),
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_args() -> Result<Args, String> {
    let mut raw = env::args().skip(1);
    let cmd = raw.next().unwrap_or_else(|| "help".to_string());
    if cmd == "help" || cmd == "--help" || cmd == "-h" {
        print_help();
        std::process::exit(0);
    }
    let mut args = Args {
        cmd,
        workspace: PathBuf::from("."),
        package: None,
        bin: None,
        bins: false,
        lib_only: false,
        example: None,
        examples: false,
        features: BTreeSet::new(),
        all_features: false,
        no_default_features: false,
        release: false,
        keep_ram: true,
        fresh: false,
        target: None,
        out: None,
        rest: Vec::new(),
    };
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--workspace" => args.workspace = PathBuf::from(need_value(&mut raw, "--workspace")?),
            "-p" | "--package" => args.package = Some(need_value(&mut raw, "--package")?),
            "--bin" => args.bin = Some(need_value(&mut raw, "--bin")?),
            arg if arg.starts_with("--bin=") => args.bin = Some(arg["--bin=".len()..].to_string()),
            "--bins" => args.bins = true,
            "--lib" => args.lib_only = true,
            "--example" => args.example = Some(need_value(&mut raw, "--example")?),
            arg if arg.starts_with("--example=") => {
                args.example = Some(arg["--example=".len()..].to_string())
            }
            "--examples" => args.examples = true,
            "--features" => {
                for feature in need_value(&mut raw, "--features")?.split(',') {
                    let feature = feature.trim();
                    if !feature.is_empty() {
                        args.features.insert(feature.to_string());
                    }
                }
            }
            "--all-features" => args.all_features = true,
            "--no-default-features" => args.no_default_features = true,
            "--release" => args.release = true,
            "--keep-ram" => args.keep_ram = true,
            "--fresh" => args.fresh = true,
            "--target" => args.target = Some(need_value(&mut raw, "--target")?),
            arg if arg.starts_with("--target=") => {
                args.target = Some(arg["--target=".len()..].to_string())
            }
            "--out" => args.out = Some(PathBuf::from(need_value(&mut raw, "--out")?)),
            "--" => {
                args.rest.extend(raw);
                break;
            }
            other => args.rest.push(other.to_string()),
        }
    }
    args.workspace = args
        .workspace
        .canonicalize()
        .map_err(|err| format!("invalid workspace path: {err}"))?;
    Ok(args)
}

fn need_value(raw: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    raw.next().ok_or_else(|| format!("{flag} requires a value"))
}

fn index_workspace(
    args: &Args,
    _root: PathBuf,
    vfs: &edgerun_vfs::VirtualFileSystem,
) -> Result<Workspace, String> {
    let root_manifest = vfs
        .read_str("Cargo.toml")
        .ok_or_else(|| "missing workspace Cargo.toml".to_string())?;
    let root_toml = parse_manifest(root_manifest);
    let members = root_toml.workspace_members;
    let mut member_dirs = BTreeSet::new();
    for item in members {
        expand_member(vfs, &item, &mut member_dirs);
    }
    let mut packages = BTreeMap::new();
    for dir in member_dirs {
        let manifest_path = format!("{dir}/Cargo.toml");
        let Some(text) = vfs.read_str(&manifest_path) else {
            continue;
        };
        let toml = parse_manifest(text);
        let Some(name) = toml.package_name.as_deref() else {
            continue;
        };
        let edition = toml.edition.clone().unwrap_or_else(|| "2021".to_string());
        let features = toml.features.clone();
        let deps = toml.deps.clone();
        let lib = target_lib(vfs, &toml, &dir, name);
        let bins = target_bins(vfs, &toml, &dir);
        let examples = target_examples(vfs, &toml, &dir);
        let host_cfg = args
            .target
            .as_deref()
            .map(HostCfg::from_target_triple)
            .unwrap_or_else(HostCfg::current);
        let build_cfgs = parse_build_cfgs(
            vfs.read_str(&format!("{dir}/build.rs")).unwrap_or(""),
            &host_cfg,
        );
        packages.insert(
            name.to_string(),
            Package {
                name: name.to_string(),
                dir,
                edition,
                features,
                deps,
                lib,
                bins,
                examples,
                build_cfgs,
            },
        );
    }
    Ok(Workspace { packages })
}

fn expand_member(vfs: &edgerun_vfs::VirtualFileSystem, pattern: &str, out: &mut BTreeSet<String>) {
    let pattern = norm(pattern);
    if !pattern.contains('*') && !pattern.contains('?') {
        if vfs.read_str(&format!("{pattern}/Cargo.toml")).is_some() {
            out.insert(pattern);
        }
        return;
    }
    for (path, _) in vfs.files() {
        if let Some(dir) = path.strip_suffix("/Cargo.toml") {
            if glob_match(&pattern, dir) {
                out.insert(dir.to_string());
            }
        }
    }
}

fn load_vfs(root_dir: &str) -> Result<edgerun_vfs::VirtualFileSystem, String> {
    let root = Path::new(root_dir);
    let mut entries = Vec::new();
    collect_vfs_entries(root, root, &mut entries)?;
    edgerun_vfs::VirtualFileSystem::from_entries_with_root(root_dir, entries)
}

fn collect_vfs_entries(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(current) else {
        return Ok(());
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if matches!(
            name,
            ".git"
                | ".next"
                | ".turbo"
                | "coverage"
                | "dist"
                | "node_modules"
                | "target"
                | ".edgerun"
        ) {
            continue;
        }
        if file_type.is_dir() {
            collect_vfs_entries(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            out.push((rel, bytes));
        }
    }
    Ok(())
}

fn glob_match(pattern: &str, text: &str) -> bool {
    fn inner(p: &[u8], t: &[u8]) -> bool {
        if p.is_empty() {
            return t.is_empty();
        }
        match p[0] {
            b'*' => inner(&p[1..], t) || (!t.is_empty() && inner(p, &t[1..])),
            b'?' => !t.is_empty() && inner(&p[1..], &t[1..]),
            ch => !t.is_empty() && ch == t[0] && inner(&p[1..], &t[1..]),
        }
    }
    inner(pattern.as_bytes(), text.as_bytes())
}

fn parse_manifest(source: &str) -> Manifest {
    let mut manifest = Manifest::default();
    let mut section = String::new();
    let mut bin: Option<Target> = None;
    let mut example: Option<Target> = None;
    let mut dep_table: Option<(String, Dep)> = None;
    let mut lines = logical_toml_lines(source);
    for line in lines.drain(..) {
        let line = strip_comment(&line).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("[[") && line.ends_with("]]") {
            flush_dep_table(&mut manifest, &mut dep_table);
            if let Some(target) = bin.take() {
                manifest.bins.push(target);
            }
            if let Some(target) = example.take() {
                manifest.examples.push(target);
            }
            section = line.trim_matches(&['[', ']'][..]).to_string();
            if section == "bin" {
                bin = Some(Target {
                    name: String::new(),
                    path: String::new(),
                    kind: TargetKind::Bin,
                    required_features: Vec::new(),
                    crate_types: Vec::new(),
                });
            } else if section == "example" {
                example = Some(Target {
                    name: String::new(),
                    path: String::new(),
                    kind: TargetKind::Example,
                    required_features: Vec::new(),
                    crate_types: Vec::new(),
                });
            }
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            flush_dep_table(&mut manifest, &mut dep_table);
            if let Some(target) = bin.take() {
                manifest.bins.push(target);
            }
            if let Some(target) = example.take() {
                manifest.examples.push(target);
            }
            section = line.trim_matches(&['[', ']'][..]).to_string();
            if let Some(dep_name) = dependency_subtable_name(&section) {
                dep_table = Some((dep_name.to_string(), default_dep(dep_name)));
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if let Some((_, dep)) = &mut dep_table {
            update_dep_field(dep, key, value);
            continue;
        }
        match section.as_str() {
            "workspace" if key == "members" => {
                manifest.workspace_members = parse_string_array(value)
            }
            "package" if key == "name" => manifest.package_name = parse_string(value),
            "package" if key == "edition" => manifest.edition = parse_string(value),
            "features" => {
                manifest
                    .features
                    .insert(key.to_string(), parse_string_array(value));
            }
            "dependencies" | "dev-dependencies" | "build-dependencies" => {
                manifest.deps.insert(key.to_string(), parse_dep(key, value));
            }
            "lib" if key == "path" => manifest.lib_path = parse_string(value),
            "lib" if key == "proc-macro" => {
                manifest.lib_proc_macro = parse_bool(value).unwrap_or(false)
            }
            "lib" if key == "crate-type" => {
                manifest.lib_proc_macro =
                    parse_string_array(value).iter().any(|v| v == "proc-macro")
            }
            "bin" if key == "name" => {
                if let Some(target) = &mut bin {
                    target.name = parse_string(value).unwrap_or_default();
                }
            }
            "bin" if key == "path" => {
                if let Some(target) = &mut bin {
                    target.path = parse_string(value).unwrap_or_default();
                }
            }
            "bin" if key == "required-features" => {
                if let Some(target) = &mut bin {
                    target.required_features = parse_string_array(value);
                }
            }
            "example" if key == "name" => {
                if let Some(target) = &mut example {
                    target.name = parse_string(value).unwrap_or_default();
                }
            }
            "example" if key == "path" => {
                if let Some(target) = &mut example {
                    target.path = parse_string(value).unwrap_or_default();
                }
            }
            "example" if key == "required-features" => {
                if let Some(target) = &mut example {
                    target.required_features = parse_string_array(value);
                }
            }
            "example" if key == "crate-type" => {
                if let Some(target) = &mut example {
                    target.crate_types = parse_string_array(value);
                }
            }
            _ => {}
        }
    }
    flush_dep_table(&mut manifest, &mut dep_table);
    if let Some(target) = bin.take() {
        manifest.bins.push(target);
    }
    if let Some(target) = example.take() {
        manifest.examples.push(target);
    }
    manifest.bins.retain(|target| !target.name.is_empty());
    manifest.examples.retain(|target| !target.name.is_empty());
    manifest
}

fn flush_dep_table(manifest: &mut Manifest, dep_table: &mut Option<(String, Dep)>) {
    if let Some((name, dep)) = dep_table.take() {
        manifest.deps.insert(name, dep);
    }
}

fn dependency_subtable_name(section: &str) -> Option<&str> {
    for prefix in ["dependencies.", "dev-dependencies.", "build-dependencies."] {
        if let Some(name) = section.strip_prefix(prefix) {
            return Some(unquote_section_segment(name));
        }
    }
    if !section.contains(".dependencies.") {
        return None;
    }
    section
        .rsplit_once(".dependencies.")
        .map(|(_, name)| unquote_section_segment(name))
}

fn unquote_section_segment(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'').trim()
}

fn update_dep_field(dep: &mut Dep, key: &str, value: &str) {
    match key {
        "package" => {
            if let Some(package) = parse_string(value) {
                dep.package = package;
            }
        }
        "optional" => dep.optional = parse_bool(value).unwrap_or(false),
        "default-features" => dep.default_features = parse_bool(value).unwrap_or(true),
        "features" => dep.features = parse_string_array(value),
        _ => {}
    }
}

fn logical_toml_lines(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    for raw in source.lines() {
        let line = raw.trim();
        if current.is_empty() {
            current.push_str(line);
        } else {
            current.push(' ');
            current.push_str(line);
        }
        for ch in strip_comment(line).chars() {
            match ch {
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
        }
        if bracket_depth <= 0 && brace_depth <= 0 {
            out.push(current.trim().to_string());
            current.clear();
            bracket_depth = 0;
            brace_depth = 0;
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

fn strip_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escape = false;
    for ch in line.chars() {
        if escape {
            out.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            out.push(ch);
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            out.push(ch);
            continue;
        }
        if ch == '#' && !in_string {
            break;
        }
        out.push(ch);
    }
    out
}

fn parse_dep(name: &str, value: &str) -> Dep {
    let mut dep = default_dep(name);
    let value = value.trim();
    if !(value.starts_with('{') && value.ends_with('}')) {
        return dep;
    }
    for (key, val) in parse_inline_table(value) {
        update_dep_field(&mut dep, &key, &val);
    }
    dep
}

fn default_dep(name: &str) -> Dep {
    Dep {
        package: name.to_string(),
        rename: crate_name(name),
        optional: false,
        default_features: true,
        features: Vec::new(),
    }
}

fn parse_inline_table(value: &str) -> Vec<(String, String)> {
    let inner = value.trim().trim_start_matches('{').trim_end_matches('}');
    split_commas(inner)
        .into_iter()
        .filter_map(|item| {
            let (key, val) = item.split_once('=')?;
            Some((key.trim().to_string(), val.trim().to_string()))
        })
        .collect()
}

fn parse_string_array(value: &str) -> Vec<String> {
    let inner = value.trim().trim_start_matches('[').trim_end_matches(']');
    split_commas(inner)
        .into_iter()
        .filter_map(|item| parse_string(item.trim()))
        .collect()
}

fn split_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_string = false;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    let bytes = value.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_string = !in_string,
            b'[' if !in_string => bracket_depth += 1,
            b']' if !in_string => bracket_depth -= 1,
            b'{' if !in_string => brace_depth += 1,
            b'}' if !in_string => brace_depth -= 1,
            b',' if !in_string && bracket_depth == 0 && brace_depth == 0 => {
                parts.push(value[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let tail = value[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

fn parse_string(value: &str) -> Option<String> {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .map(str::to_string)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn target_lib(
    vfs: &edgerun_vfs::VirtualFileSystem,
    toml: &Manifest,
    dir: &str,
    package_name: &str,
) -> Option<Target> {
    let path = toml
        .lib_path
        .as_deref()
        .map(|p| format!("{dir}/{}", norm(p)))
        .unwrap_or_else(|| format!("{dir}/src/lib.rs"));
    vfs.read_str(&path).map(|_| Target {
        name: crate_name(package_name),
        path,
        kind: if toml.lib_proc_macro {
            TargetKind::ProcMacro
        } else {
            TargetKind::Lib
        },
        required_features: Vec::new(),
        crate_types: Vec::new(),
    })
}

fn target_bins(vfs: &edgerun_vfs::VirtualFileSystem, toml: &Manifest, dir: &str) -> Vec<Target> {
    let mut bins = toml
        .bins
        .iter()
        .filter_map(|bin| {
            let path = if bin.path.is_empty() {
                format!("{dir}/src/bin/{}.rs", bin.name)
            } else {
                format!("{dir}/{}", norm(&bin.path))
            };
            vfs.read_str(&path).map(|_| Target {
                name: crate_name(&bin.name),
                path,
                kind: TargetKind::Bin,
                required_features: bin.required_features.clone(),
                crate_types: Vec::new(),
            })
        })
        .collect::<Vec<_>>();
    let main = format!("{dir}/src/main.rs");
    if vfs.read_str(&main).is_some() {
        bins.push(Target {
            name: "main".to_string(),
            path: main,
            kind: TargetKind::Bin,
            required_features: Vec::new(),
            crate_types: Vec::new(),
        });
    }
    bins.sort_by(|a, b| a.name.cmp(&b.name));
    bins.dedup_by(|a, b| a.name == b.name);
    bins
}

fn target_examples(
    vfs: &edgerun_vfs::VirtualFileSystem,
    toml: &Manifest,
    dir: &str,
) -> Vec<Target> {
    let mut examples = toml
        .examples
        .iter()
        .filter_map(|example| {
            let path = if example.path.is_empty() {
                format!("{dir}/examples/{}.rs", example.name)
            } else {
                format!("{dir}/{}", norm(&example.path))
            };
            vfs.read_str(&path).map(|_| Target {
                name: crate_name(&example.name),
                path,
                kind: TargetKind::Example,
                required_features: example.required_features.clone(),
                crate_types: example.crate_types.clone(),
            })
        })
        .collect::<Vec<_>>();
    examples.sort_by(|a, b| a.name.cmp(&b.name));
    examples.dedup_by(|a, b| a.name == b.name);
    examples
}

fn select_package(args: &Args, ws: &Workspace) -> Result<String, String> {
    if let Some(pkg) = &args.package {
        if ws.packages.contains_key(pkg) {
            return Ok(pkg.clone());
        }
        return Err(format!("package not found: {pkg}"));
    }
    if ws.packages.len() == 1 {
        return Ok(ws.packages.keys().next().unwrap().clone());
    }
    Err("choose a package with -p".to_string())
}

fn resolve_closure(ws: &Workspace, root: &str, args: &Args) -> Result<Vec<BuildNode>, String> {
    let mut features_by_pkg: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut queue = VecDeque::new();
    let mut root_features = args.features.clone();
    if args.all_features {
        root_features.extend(ws.packages[root].features.keys().cloned());
    } else if !args.no_default_features {
        root_features.insert("default".to_string());
    }
    queue.push_back((root.to_string(), root_features));
    while let Some((pkg_name, mut features)) = queue.pop_front() {
        let pkg = ws
            .packages
            .get(&pkg_name)
            .ok_or_else(|| format!("unknown package: {pkg_name}"))?;
        expand_package_features(pkg, &mut features);
        let entry = features_by_pkg.entry(pkg_name.clone()).or_default();
        let before = entry.len();
        entry.extend(features.iter().cloned());
        if entry.len() == before && before != 0 {
            continue;
        }
        for (dep_key, dep) in &pkg.deps {
            if dep.optional && !dep_is_enabled(dep_key, &features) {
                continue;
            }
            if !ws.packages.contains_key(&dep.package) {
                continue;
            }
            let mut dep_features = BTreeSet::new();
            if dep.default_features {
                dep_features.insert("default".to_string());
            }
            dep_features.extend(dep.features.iter().cloned());
            for feature in &features {
                if let Some(suffix) = feature.strip_prefix(&format!("{dep_key}/")) {
                    dep_features.insert(suffix.to_string());
                }
            }
            queue.push_back((dep.package.clone(), dep_features));
        }
    }
    let mut ordered = Vec::new();
    topo(
        root,
        ws,
        &features_by_pkg,
        &mut BTreeSet::new(),
        &mut ordered,
    )?;
    Ok(ordered
        .into_iter()
        .map(|package| BuildNode {
            enabled_features: features_by_pkg.remove(&package).unwrap_or_default(),
            package,
        })
        .collect())
}

fn expand_package_features(pkg: &Package, features: &mut BTreeSet<String>) {
    let mut queue: VecDeque<String> = features.iter().cloned().collect();
    while let Some(feature) = queue.pop_front() {
        let Some(items) = pkg.features.get(&feature) else {
            continue;
        };
        for item in items {
            if let Some(dep) = item.strip_prefix("dep:") {
                if features.insert(format!("dep:{dep}")) {
                    queue.push_back(format!("dep:{dep}"));
                }
            } else if item.contains('/') {
                features.insert(item.clone());
            } else if !item.contains('/') && features.insert(item.clone()) {
                queue.push_back(item.clone());
            }
        }
    }
}

fn dep_is_enabled(dep_key: &str, features: &BTreeSet<String>) -> bool {
    features.contains(dep_key) || features.contains(&format!("dep:{dep_key}"))
}

fn topo(
    pkg_name: &str,
    ws: &Workspace,
    features_by_pkg: &BTreeMap<String, BTreeSet<String>>,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<String>,
) -> Result<(), String> {
    if !seen.insert(pkg_name.to_string()) {
        return Ok(());
    }
    let pkg = ws
        .packages
        .get(pkg_name)
        .ok_or_else(|| format!("unknown package: {pkg_name}"))?;
    let features = features_by_pkg.get(pkg_name).cloned().unwrap_or_default();
    for (dep_key, dep) in &pkg.deps {
        if dep.optional && !dep_is_enabled(dep_key, &features) {
            continue;
        }
        if features_by_pkg.contains_key(&dep.package) {
            topo(&dep.package, ws, features_by_pkg, seen, out)?;
        }
    }
    out.push(pkg_name.to_string());
    Ok(())
}

fn build_command(
    args: &Args,
    ws: &Workspace,
    vfs: &edgerun_vfs::VirtualFileSystem,
) -> Result<(), String> {
    let root = select_package(args, ws)?;
    let nodes = resolve_closure(ws, &root, args)?;
    validate_requested_targets(args, ws, &root, &nodes)?;
    let ram = ram_root(args)?;
    if ram.exists() && args.fresh {
        let _ = fs::remove_dir_all(&ram);
    }
    fs::create_dir_all(&ram).map_err(|err| format!("mkdir {}: {err}", ram.display()))?;
    let src_root = ram.join("src");
    let out_dir = ram.join(if args.release { "release" } else { "debug" });
    fs::create_dir_all(&src_root).map_err(|err| format!("mkdir {}: {err}", src_root.display()))?;
    fs::create_dir_all(&out_dir).map_err(|err| format!("mkdir {}: {err}", out_dir.display()))?;
    materialize_closure(vfs, ws, &nodes, &src_root)?;

    let mut artifacts = BTreeMap::new();
    for node in &nodes {
        let pkg = &ws.packages[&node.package];
        let is_root = node.package == root;
        if let Some(lib) = &pkg.lib {
            let artifact = compile_target(
                args,
                pkg,
                lib,
                node,
                &src_root,
                &out_dir,
                &artifacts,
                args.cmd == "check" && is_root,
            )?;
            artifacts.insert(pkg.name.clone(), artifact);
        }
        if is_root && should_compile_bins(args) {
            for bin in &pkg.bins {
                if !bin_selected(args, bin) {
                    continue;
                }
                if !target_required_features_met(bin, &node.enabled_features) {
                    continue;
                }
                let artifact =
                    compile_target(args, pkg, bin, node, &src_root, &out_dir, &artifacts, false)?;
                if args.cmd == "run" {
                    if let Some(target) = &args.target {
                        return Err(format!(
                            "cannot run cross-compiled target {target}; add runner support to edgerun-build"
                        ));
                    }
                    let status = Command::new(&artifact.path)
                        .args(&args.rest)
                        .status()
                        .map_err(|err| format!("run {}: {err}", artifact.path.display()))?;
                    if !status.success() {
                        return Err(format!("program exited with {status}"));
                    }
                }
            }
        }
        if is_root && args.cmd == "build" && examples_requested(args) {
            for example in &pkg.examples {
                if !example_selected(args, example) {
                    continue;
                }
                compile_target(
                    args, pkg, example, node, &src_root, &out_dir, &artifacts, false,
                )?;
            }
        }
    }
    if args.cmd == "test" {
        compile_tests(args, ws, &root, &nodes, &src_root, &out_dir, &artifacts)?;
    }
    if let Some(out) = &args.out {
        fs::create_dir_all(out).map_err(|err| format!("mkdir {}: {err}", out.display()))?;
        for entry in fs::read_dir(&out_dir).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_file() {
                let dest = out.join(path.file_name().unwrap());
                copy_if_changed(&path, &dest)?;
            }
        }
    }
    if !args.keep_ram {
        let _ = fs::remove_dir_all(&ram);
    }
    Ok(())
}

fn materialize_closure(
    vfs: &edgerun_vfs::VirtualFileSystem,
    ws: &Workspace,
    nodes: &[BuildNode],
    src_root: &Path,
) -> Result<(), String> {
    let dirs = nodes
        .iter()
        .map(|node| ws.packages[&node.package].dir.clone())
        .collect::<Vec<_>>();
    let mut current_files = BTreeSet::new();
    for (path, bytes) in vfs.files() {
        if dirs
            .iter()
            .any(|dir| path == dir || path.starts_with(&format!("{dir}/")))
        {
            current_files.insert(path.clone());
            let dest = src_root.join(path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("mkdir {}: {err}", parent.display()))?;
            }
            write_if_changed(&dest, bytes)?;
        }
    }
    remove_stale_materialized_files(src_root, &current_files)?;
    Ok(())
}

fn remove_stale_materialized_files(
    src_root: &Path,
    current_files: &BTreeSet<String>,
) -> Result<(), String> {
    let manifest = src_root.join(".edgerun-source-manifest");
    if let Ok(previous) = fs::read_to_string(&manifest) {
        for path in previous.lines().filter(|line| !line.is_empty()) {
            if !current_files.contains(path) {
                let stale = src_root.join(path);
                if stale.is_file() {
                    fs::remove_file(&stale)
                        .map_err(|err| format!("remove {}: {err}", stale.display()))?;
                }
            }
        }
    }
    let mut next = String::new();
    for path in current_files {
        next.push_str(path);
        next.push('\n');
    }
    write_if_changed(&manifest, next.as_bytes())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    fs::write(path, bytes).map_err(|err| format!("write {}: {err}", path.display()))
}

fn copy_if_changed(source: &Path, dest: &Path) -> Result<(), String> {
    let source_bytes =
        fs::read(source).map_err(|err| format!("read {}: {err}", source.display()))?;
    if fs::read(dest).is_ok_and(|existing| existing == source_bytes) {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir {}: {err}", parent.display()))?;
    }
    fs::write(dest, source_bytes)
        .map_err(|err| format!("copy {} -> {}: {err}", source.display(), dest.display()))
}

fn validate_requested_targets(
    args: &Args,
    ws: &Workspace,
    root: &str,
    nodes: &[BuildNode],
) -> Result<(), String> {
    let pkg = &ws.packages[root];
    let node = nodes
        .iter()
        .find(|node| node.package == root)
        .ok_or_else(|| "root package missing from build graph".to_string())?;

    if let Some(bin) = &args.bin {
        let Some(target) = pkg.bins.iter().find(|target| bin_selected(args, target)) else {
            return Err(format!("bin not found: {bin}"));
        };
        if !target_required_features_met(target, &node.enabled_features) {
            return Err(format!(
                "bin {} requires features: {}",
                target.name,
                target.required_features.join(",")
            ));
        }
    }

    if args.cmd == "run" {
        let selected = pkg
            .bins
            .iter()
            .filter(|bin| bin_selected(args, bin))
            .count();
        if selected != 1 {
            return Err(format!(
                "cargo run requires exactly one binary; selected {selected}. Use --bin NAME."
            ));
        }
    }

    if examples_requested(args) {
        let mut matched = false;
        for example in &pkg.examples {
            if !example_selected(args, example) {
                continue;
            }
            matched = true;
            if !target_required_features_met(example, &node.enabled_features) {
                return Err(format!(
                    "example {} requires features: {}",
                    example.name,
                    example.required_features.join(",")
                ));
            }
        }
        if args.example.is_some() && !matched {
            return Err(format!(
                "example not found: {}",
                args.example.as_deref().unwrap_or_default()
            ));
        }
    }

    Ok(())
}

fn compile_target(
    args: &Args,
    pkg: &Package,
    target: &Target,
    node: &BuildNode,
    src_root: &Path,
    out_dir: &Path,
    artifacts: &BTreeMap<String, BuiltArtifact>,
    metadata_only: bool,
) -> Result<BuiltArtifact, String> {
    let src = src_root.join(&target.path);
    let target_crate_name = crate_name(&target.name);
    let fingerprint =
        compile_fingerprint(args, pkg, target, node, src_root, artifacts, metadata_only)?;
    let stamp_path = out_dir.join(".edgerun-stamps").join(format!(
        "{}-{:?}-{metadata_only}.stamp",
        target_crate_name, target.kind
    ));
    if !args.fresh && fs::read_to_string(&stamp_path).is_ok_and(|stamp| stamp == fingerprint) {
        if let Ok(path) = find_artifact(out_dir, &target_crate_name, target.kind, metadata_only) {
            eprintln!("= reuse {}", path.display());
            return Ok(BuiltArtifact { path });
        }
    }
    let mut cmd = Command::new("rustc");
    cmd.arg("--crate-name")
        .arg(&target_crate_name)
        .arg(&src)
        .arg("--edition")
        .arg(&pkg.edition)
        .arg("--out-dir")
        .arg(out_dir)
        .arg("-L")
        .arg(format!("dependency={}", out_dir.display()))
        .arg("--color")
        .arg("never");
    if let Some(target_triple) = &args.target {
        if target.kind != TargetKind::ProcMacro {
            cmd.arg("--target").arg(target_triple);
        }
    }
    match target.kind {
        TargetKind::Lib => {
            cmd.arg("--crate-type").arg("lib");
            if metadata_only {
                cmd.arg("--emit").arg("metadata");
            }
        }
        TargetKind::ProcMacro => {
            cmd.arg("--crate-type").arg("proc-macro");
            cmd.arg("--extern").arg("proc_macro");
        }
        TargetKind::Example => {
            for crate_type in target
                .crate_types
                .iter()
                .map(String::as_str)
                .filter(|crate_type| !crate_type.is_empty())
            {
                cmd.arg("--crate-type").arg(crate_type);
            }
            if target.crate_types.is_empty() {
                cmd.arg("--crate-type").arg("bin");
            }
        }
        TargetKind::Bin => {}
    }
    if args.release {
        cmd.arg("-O");
    }
    for feature in &node.enabled_features {
        if !feature.starts_with("dep:") {
            cmd.arg("--cfg").arg(format!("feature=\"{feature}\""));
        }
    }
    for cfg in &pkg.build_cfgs {
        cmd.arg("--cfg").arg(cfg);
    }
    for dep in pkg.deps.values() {
        if let Some(artifact) = artifacts.get(&dep.package) {
            cmd.arg("--extern")
                .arg(format!("{}={}", dep.rename, artifact.path.display()));
        }
    }
    if matches!(target.kind, TargetKind::Bin | TargetKind::Example) {
        if let Some(artifact) = artifacts.get(&pkg.name) {
            cmd.arg("--extern").arg(format!(
                "{}={}",
                crate_name(&pkg.name),
                artifact.path.display()
            ));
        }
    }
    run_rustc(&mut cmd)?;
    let path = find_artifact(out_dir, &target_crate_name, target.kind, metadata_only)?;
    if let Some(parent) = stamp_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir {}: {err}", parent.display()))?;
    }
    fs::write(&stamp_path, fingerprint)
        .map_err(|err| format!("write {}: {err}", stamp_path.display()))?;
    Ok(BuiltArtifact { path })
}

fn compile_fingerprint(
    args: &Args,
    pkg: &Package,
    target: &Target,
    node: &BuildNode,
    src_root: &Path,
    artifacts: &BTreeMap<String, BuiltArtifact>,
    metadata_only: bool,
) -> Result<String, String> {
    let mut hasher = DefaultHasher::new();
    "edgerun-build-v2".hash(&mut hasher);
    args.target.hash(&mut hasher);
    args.release.hash(&mut hasher);
    metadata_only.hash(&mut hasher);
    pkg.name.hash(&mut hasher);
    pkg.edition.hash(&mut hasher);
    target.name.hash(&mut hasher);
    target.path.hash(&mut hasher);
    target.kind.hash(&mut hasher);
    target.required_features.hash(&mut hasher);
    target.crate_types.hash(&mut hasher);
    node.enabled_features.hash(&mut hasher);
    pkg.build_cfgs.hash(&mut hasher);
    hash_dir(&src_root.join(&pkg.dir), &mut hasher)?;
    for (name, artifact) in artifacts {
        name.hash(&mut hasher);
        artifact.path.hash(&mut hasher);
        let meta = fs::metadata(&artifact.path)
            .map_err(|err| format!("metadata {}: {err}", artifact.path.display()))?;
        meta.len().hash(&mut hasher);
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    Ok(format!("{:016x}", hasher.finish()))
}

fn hash_dir(path: &Path, hasher: &mut DefaultHasher) -> Result<(), String> {
    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(path) else {
        return Ok(());
    };
    for entry in read_dir {
        let entry = entry.map_err(|err| err.to_string())?;
        entries.push(entry.path());
    }
    entries.sort();
    for path in entries {
        let meta =
            fs::metadata(&path).map_err(|err| format!("metadata {}: {err}", path.display()))?;
        path.hash(hasher);
        if meta.is_dir() {
            hash_dir(&path, hasher)?;
        } else if meta.is_file() {
            meta.len().hash(hasher);
            let bytes = fs::read(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
            bytes.hash(hasher);
        }
    }
    Ok(())
}

fn compile_tests(
    args: &Args,
    ws: &Workspace,
    root: &str,
    nodes: &[BuildNode],
    src_root: &Path,
    out_dir: &Path,
    artifacts: &BTreeMap<String, BuiltArtifact>,
) -> Result<(), String> {
    if args.target.as_deref() == Some("wasm32-unknown-unknown") {
        return Err("wasm32-unknown-unknown test execution is not supported yet; add wasm test harness support to edgerun-build".to_string());
    }
    let node = nodes
        .iter()
        .find(|node| node.package == root)
        .ok_or_else(|| "root package missing from build graph".to_string())?;
    let pkg = &ws.packages[root];
    let Some(lib) = &pkg.lib else {
        return Ok(());
    };
    let test_bin = out_dir.join(format!("{}_tests", crate_name(root)));
    let mut cmd = Command::new("rustc");
    cmd.arg("--test")
        .arg(src_root.join(&lib.path))
        .arg("--crate-name")
        .arg(crate_name(root))
        .arg("--edition")
        .arg(&pkg.edition)
        .arg("-o")
        .arg(&test_bin)
        .arg("-L")
        .arg(format!("dependency={}", out_dir.display()))
        .arg("--color")
        .arg("never");
    for feature in &node.enabled_features {
        if !feature.starts_with("dep:") {
            cmd.arg("--cfg").arg(format!("feature=\"{feature}\""));
        }
    }
    for cfg in &pkg.build_cfgs {
        cmd.arg("--cfg").arg(cfg);
    }
    for dep in pkg.deps.values() {
        if let Some(artifact) = artifacts.get(&dep.package) {
            cmd.arg("--extern")
                .arg(format!("{}={}", dep.rename, artifact.path.display()));
        }
    }
    run_rustc(&mut cmd)?;
    let status = Command::new(&test_bin)
        .status()
        .map_err(|err| format!("run tests {}: {err}", test_bin.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("tests failed with {status}"))
    }
}

fn run_rustc(cmd: &mut Command) -> Result<(), String> {
    eprintln!("+ {:?}", cmd);
    let status = cmd
        .stdin(Stdio::null())
        .status()
        .map_err(|err| format!("rustc failed to start: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("rustc exited with {status}"))
    }
}

fn find_artifact(
    out_dir: &Path,
    crate_name: &str,
    kind: TargetKind,
    metadata: bool,
) -> Result<PathBuf, String> {
    let prefix = match (kind, metadata) {
        (TargetKind::Lib, true) => format!("lib{crate_name}.rmeta"),
        (TargetKind::Lib, false) => format!("lib{crate_name}.rlib"),
        (TargetKind::ProcMacro, _) => proc_macro_artifact_prefix(crate_name),
        (TargetKind::Bin | TargetKind::Example, _) => crate_name.to_string(),
    };
    for entry in fs::read_dir(out_dir).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| {
                name == prefix
                    || name == format!("{prefix}.wasm")
                    || name.starts_with(&format!("{prefix}-"))
            })
        {
            return Ok(path);
        }
    }
    Err(format!("artifact not found for {crate_name}"))
}

fn target_required_features_met(target: &Target, features: &BTreeSet<String>) -> bool {
    target
        .required_features
        .iter()
        .all(|feature| features.contains(feature))
}

fn should_compile_bins(args: &Args) -> bool {
    matches!(args.cmd.as_str(), "build" | "run" | "test")
        && !examples_requested(args)
        && (!args.lib_only || args.bin.is_some() || args.bins)
}

fn bin_selected(args: &Args, target: &Target) -> bool {
    match args.bin.as_deref() {
        Some(name) => crate_name(name) == target.name,
        None => true,
    }
}

fn examples_requested(args: &Args) -> bool {
    args.examples || args.example.is_some()
}

fn example_selected(args: &Args, target: &Target) -> bool {
    args.examples
        || args
            .example
            .as_deref()
            .is_some_and(|name| crate_name(name) == target.name)
}

fn proc_macro_artifact_prefix(crate_name: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("lib{crate_name}.dylib")
    } else if cfg!(target_os = "windows") {
        format!("{crate_name}.dll")
    } else {
        format!("lib{crate_name}.so")
    }
}

fn parse_build_cfgs(source: &str, host: &HostCfg) -> Vec<String> {
    let mut out = Vec::new();
    if source.contains("procmacro2_semver_exempt") {
        return out;
    }
    if source.contains("curve25519_dalek_bits") {
        out.push(format!(
            "curve25519_dalek_bits=\"{}\"",
            if host.pointer_width == "64" {
                "64"
            } else {
                "32"
            }
        ));
    }
    if source.contains("curve25519_dalek_backend") {
        out.push("curve25519_dalek_backend=\"serial\"".to_string());
    }
    if source.contains("has_i128") && host.os != "none" {
        out.push("has_i128".to_string());
    }
    if source.contains("arch_enabled") {
        out.push("arch_enabled".to_string());
    }
    if source.contains("x86_no_sse") && host.arch == "x86" {
        out.push("x86_no_sse".to_string());
    }
    for line in source.lines().map(str::trim) {
        if let Some(cfg) = literal_rustc_cfg(line) {
            if cfg != "x86_no_sse" {
                out.push(cfg);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn literal_rustc_cfg(line: &str) -> Option<String> {
    let pos = line.find("cargo:rustc-cfg=")?;
    if line.contains('{') || line.contains("format!") || line.starts_with("//") {
        return None;
    }
    let cfg = line[pos + "cargo:rustc-cfg=".len()..]
        .trim_matches(|ch| matches!(ch, '"' | '\'' | ')' | ';' | ' ' | '\t'))
        .to_string();
    (!cfg.is_empty()).then_some(cfg)
}

fn print_graph(ws: &Workspace, nodes: &[BuildNode]) {
    for node in nodes {
        let pkg = &ws.packages[&node.package];
        println!(
            "{}\t{}\tfeatures={}",
            pkg.name,
            pkg.dir,
            node.enabled_features
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        );
    }
}

fn doctor(args: &Args) -> Result<(), String> {
    println!("workspace={}", args.workspace.display());
    println!("ram-root={}", ram_root(args)?.display());
    println!("ram-reuse={}", if args.fresh { "fresh" } else { "enabled" });
    println!("disk-cache=none");
    println!("cargo-metadata=never");
    println!("home-writes=none");
    println!(
        "final-output={}",
        args.out
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    Ok(())
}

fn clean_ram() -> Result<(), String> {
    let root = PathBuf::from("/dev/shm/edgerun-build");
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|err| format!("remove {}: {err}", root.display()))?;
    }
    Ok(())
}

fn ram_root(args: &Args) -> Result<PathBuf, String> {
    let base = if Path::new("/dev/shm").is_dir() {
        PathBuf::from("/dev/shm/edgerun-build")
    } else {
        env::temp_dir().join("edgerun-build")
    };
    let ws = args
        .workspace
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace");
    let mut hasher = DefaultHasher::new();
    args.workspace.hash(&mut hasher);
    args.package.hash(&mut hasher);
    args.bin.hash(&mut hasher);
    args.bins.hash(&mut hasher);
    args.lib_only.hash(&mut hasher);
    args.example.hash(&mut hasher);
    args.examples.hash(&mut hasher);
    args.features.hash(&mut hasher);
    args.all_features.hash(&mut hasher);
    args.no_default_features.hash(&mut hasher);
    args.release.hash(&mut hasher);
    args.target.hash(&mut hasher);
    Ok(base.join(format!("{ws}-{:016x}", hasher.finish())))
}

fn crate_name(name: &str) -> String {
    name.replace('-', "_")
}

fn norm(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_matches('/')
        .replace('\\', "/")
}

fn path_str(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("path is not utf-8: {}", path.display()))
}

fn print_help() {
    println!(
        "edgerun-build <doctor|graph|check|build|test|run|clean> [options]\n\
         options: -p NAME --lib --bin NAME --bins --example NAME --examples --features a,b --all-features --no-default-features --release --target TRIPLE --workspace PATH --out PATH --keep-ram --fresh"
    );
}
