use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

extern crate edgerun_json;
extern crate edgerun_vfs;

use edgerun_json::TomlValue;

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
    features: BTreeSet<String>,
    all_features: bool,
    no_default_features: bool,
    release: bool,
    keep_ram: bool,
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
    build_cfgs: Vec<String>,
}

#[derive(Debug, Clone)]
struct Target {
    name: String,
    path: String,
    kind: TargetKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetKind {
    Lib,
    Bin,
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

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let vfs = load_vfs(path_str(&args.workspace)?)?;
    let ws = index_workspace(args.workspace.clone(), &vfs)?;
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
        features: BTreeSet::new(),
        all_features: false,
        no_default_features: false,
        release: false,
        keep_ram: false,
        out: None,
        rest: Vec::new(),
    };
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--workspace" => args.workspace = PathBuf::from(need_value(&mut raw, "--workspace")?),
            "-p" | "--package" => args.package = Some(need_value(&mut raw, "--package")?),
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
    _root: PathBuf,
    vfs: &edgerun_vfs::VirtualFileSystem,
) -> Result<Workspace, String> {
    let root_manifest = vfs
        .read_str("Cargo.toml")
        .ok_or_else(|| "missing workspace Cargo.toml".to_string())?;
    let root_toml = edgerun_json::from_toml_str(root_manifest)
        .map_err(|err| format!("parse Cargo.toml: {err}"))?;
    let members = root_toml
        .get("workspace")
        .and_then(|v| v.get("members"))
        .and_then(TomlValue::as_array)
        .ok_or_else(|| "workspace members not found".to_string())?;
    let mut member_dirs = BTreeSet::new();
    for item in members {
        if let Some(pattern) = item.as_str() {
            expand_member(vfs, pattern, &mut member_dirs);
        }
    }
    let mut packages = BTreeMap::new();
    for dir in member_dirs {
        let manifest_path = format!("{dir}/Cargo.toml");
        let Some(text) = vfs.read_str(&manifest_path) else {
            continue;
        };
        let toml = match edgerun_json::from_toml_str(text) {
            Ok(toml) => toml,
            Err(_) => continue,
        };
        let Some(package) = toml.get("package") else {
            continue;
        };
        let Some(name) = package.get("name").and_then(TomlValue::as_str) else {
            continue;
        };
        let edition = package
            .get("edition")
            .and_then(TomlValue::as_str)
            .unwrap_or("2021")
            .to_string();
        let features = parse_features(&toml);
        let deps = parse_deps(&toml);
        let lib = target_lib(vfs, &toml, &dir, name);
        let bins = target_bins(vfs, &toml, &dir);
        let build_cfgs = parse_build_cfgs(vfs.read_str(&format!("{dir}/build.rs")).unwrap_or(""));
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
    for entry in
        fs::read_dir(current).map_err(|err| format!("read {}: {err}", current.display()))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
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
        if path.is_dir() {
            collect_vfs_entries(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
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

fn parse_features(toml: &TomlValue) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    if let Some(table) = toml.get("features").and_then(TomlValue::as_table) {
        for (name, value) in table {
            let items = value
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            out.insert(name.clone(), items);
        }
    }
    out
}

fn parse_deps(toml: &TomlValue) -> BTreeMap<String, Dep> {
    let mut out = BTreeMap::new();
    for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
        if let Some(table) = toml.get(section).and_then(TomlValue::as_table) {
            for (name, value) in table {
                let mut dep = Dep {
                    package: name.clone(),
                    rename: crate_name(name),
                    optional: false,
                    default_features: true,
                    features: Vec::new(),
                };
                if let Some(dep_table) = value.as_table() {
                    if let Some(pkg) = table_value_str(dep_table, "package") {
                        dep.package = pkg.to_string();
                    }
                    if let Some(optional) =
                        table_value(dep_table, "optional").and_then(TomlValue::as_bool)
                    {
                        dep.optional = optional;
                    }
                    if let Some(default_features) =
                        table_value(dep_table, "default-features").and_then(TomlValue::as_bool)
                    {
                        dep.default_features = default_features;
                    }
                    if let Some(features) =
                        table_value(dep_table, "features").and_then(TomlValue::as_array)
                    {
                        dep.features = features
                            .iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect();
                    }
                }
                out.insert(name.clone(), dep);
            }
        }
    }
    out
}

fn table_value<'a>(table: &'a [(String, TomlValue)], key: &str) -> Option<&'a TomlValue> {
    table.iter().find_map(|(k, v)| (k == key).then_some(v))
}

fn table_value_str<'a>(table: &'a [(String, TomlValue)], key: &str) -> Option<&'a str> {
    table_value(table, key).and_then(TomlValue::as_str)
}

fn target_lib(
    vfs: &edgerun_vfs::VirtualFileSystem,
    toml: &TomlValue,
    dir: &str,
    package_name: &str,
) -> Option<Target> {
    let path = toml
        .get("lib")
        .and_then(|v| v.get("path"))
        .and_then(TomlValue::as_str)
        .map(|p| format!("{dir}/{}", norm(p)))
        .unwrap_or_else(|| format!("{dir}/src/lib.rs"));
    vfs.read_str(&path).map(|_| Target {
        name: crate_name(package_name),
        path,
        kind: TargetKind::Lib,
    })
}

fn target_bins(vfs: &edgerun_vfs::VirtualFileSystem, toml: &TomlValue, dir: &str) -> Vec<Target> {
    let mut bins = Vec::new();
    if let Some(bin) = toml.get("bin") {
        if let Some(arr) = bin.as_array() {
            for item in arr {
                if let Some(table) = item.as_table() {
                    let Some(name) = table_value_str(table, "name") else {
                        continue;
                    };
                    let path = table_value_str(table, "path")
                        .map(|p| format!("{dir}/{}", norm(p)))
                        .unwrap_or_else(|| format!("{dir}/src/bin/{name}.rs"));
                    if vfs.read_str(&path).is_some() {
                        bins.push(Target {
                            name: crate_name(name),
                            path,
                            kind: TargetKind::Bin,
                        });
                    }
                }
            }
        }
    }
    let main = format!("{dir}/src/main.rs");
    if vfs.read_str(&main).is_some() {
        bins.push(Target {
            name: "main".to_string(),
            path: main,
            kind: TargetKind::Bin,
        });
    }
    bins.sort_by(|a, b| a.name.cmp(&b.name));
    bins.dedup_by(|a, b| a.name == b.name);
    bins
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
    let ram = ram_root(args)?;
    if ram.exists() && !args.keep_ram {
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
            let artifact = compile_target(args, pkg, lib, node, &src_root, &out_dir, &artifacts)?;
            artifacts.insert(pkg.name.clone(), artifact);
        }
        if is_root && matches!(args.cmd.as_str(), "build" | "run" | "test") {
            for bin in &pkg.bins {
                let artifact =
                    compile_target(args, pkg, bin, node, &src_root, &out_dir, &artifacts)?;
                if args.cmd == "run" {
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
                fs::copy(&path, &dest).map_err(|err| {
                    format!("copy {} -> {}: {err}", path.display(), dest.display())
                })?;
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
    for (path, bytes) in vfs.files() {
        if dirs
            .iter()
            .any(|dir| path == dir || path.starts_with(&format!("{dir}/")))
        {
            let dest = src_root.join(path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("mkdir {}: {err}", parent.display()))?;
            }
            fs::write(&dest, bytes).map_err(|err| format!("write {}: {err}", dest.display()))?;
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
) -> Result<BuiltArtifact, String> {
    let src = src_root.join(&target.path);
    let crate_name = crate_name(&target.name);
    let mut cmd = Command::new("rustc");
    cmd.arg("--crate-name")
        .arg(&crate_name)
        .arg(&src)
        .arg("--edition")
        .arg(&pkg.edition)
        .arg("--out-dir")
        .arg(out_dir)
        .arg("-L")
        .arg(format!("dependency={}", out_dir.display()))
        .arg("--color")
        .arg("never");
    match target.kind {
        TargetKind::Lib => {
            cmd.arg("--crate-type").arg("lib");
            if args.cmd == "check" {
                cmd.arg("--emit").arg("metadata");
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
    run_rustc(&mut cmd)?;
    let path = find_artifact(out_dir, &crate_name, target.kind, args.cmd == "check")?;
    Ok(BuiltArtifact { path })
}

fn compile_tests(
    _args: &Args,
    ws: &Workspace,
    root: &str,
    nodes: &[BuildNode],
    src_root: &Path,
    out_dir: &Path,
    artifacts: &BTreeMap<String, BuiltArtifact>,
) -> Result<(), String> {
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
        (TargetKind::Bin, _) => crate_name.to_string(),
    };
    for entry in fs::read_dir(out_dir).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name == prefix || name.starts_with(&format!("{prefix}-")))
        {
            return Ok(path);
        }
    }
    Err(format!("artifact not found for {crate_name}"))
}

fn parse_build_cfgs(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        if line.contains("curve25519_dalek_bits") {
            out.push("curve25519_dalek_bits=\"64\"".to_string());
        } else if line.contains("curve25519_dalek_backend") {
            out.push("curve25519_dalek_backend=\"serial\"".to_string());
        } else if line.contains("has_i128") {
            out.push("has_i128".to_string());
        } else if line.contains("arch_enabled") {
            out.push("arch_enabled".to_string());
        } else if let Some(pos) = line.find("cargo:rustc-cfg=") {
            let cfg = line[pos + "cargo:rustc-cfg=".len()..]
                .trim_matches(|ch| matches!(ch, '"' | '\'' | ')' | ';' | ' ' | '\t'))
                .to_string();
            if !cfg.is_empty() && !cfg.contains('{') {
                out.push(cfg);
            }
        }
    }
    out.sort();
    out.dedup();
    out
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
    Ok(base.join(format!("{ws}-{}", std::process::id())))
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
         options: -p NAME --features a,b --all-features --no-default-features --release --workspace PATH --out PATH --keep-ram"
    );
}
