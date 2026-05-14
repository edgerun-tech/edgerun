use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;

#[derive(Clone, Debug)]
struct Options {
    apk: String,
    all: bool,
    json: bool,
    summary: bool,
    profile: bool,
    spec: bool,
    top: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ApiCall {
    class_name: String,
    method_name: String,
    args: Vec<String>,
    return_type: String,
}

#[derive(Clone, Debug)]
struct ZipEntry {
    name: String,
    compression: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    local_header_offset: usize,
}

#[derive(Clone, Debug)]
struct DexHeader {
    string_ids_size: usize,
    string_ids_off: usize,
    type_ids_size: usize,
    type_ids_off: usize,
    proto_ids_size: usize,
    proto_ids_off: usize,
    method_ids_size: usize,
    method_ids_off: usize,
    class_defs_size: usize,
    class_defs_off: usize,
}

#[derive(Clone, Debug)]
struct MethodRef {
    class_name: String,
    method_name: String,
    args: Vec<String>,
    return_type: String,
}

#[derive(Clone, Debug)]
struct Signal {
    name: &'static str,
    prefixes: &'static [&'static str],
    keywords: &'static [&'static str],
}

#[derive(Clone, Debug, Default)]
struct ApkProfile {
    manifest: ManifestProfile,
    native_libs: BTreeSet<String>,
    domains: BTreeSet<String>,
    dex_packages: BTreeMap<String, u64>,
    url_count: usize,
}

#[derive(Clone, Debug, Default)]
struct ManifestProfile {
    package_name: Option<String>,
    permissions: BTreeSet<String>,
    activities: Vec<ComponentProfile>,
    services: Vec<ComponentProfile>,
    receivers: Vec<ComponentProfile>,
    providers: Vec<ComponentProfile>,
}

#[derive(Clone, Debug, Default)]
struct ComponentProfile {
    name: String,
    exported: Option<bool>,
    actions: BTreeSet<String>,
    categories: BTreeSet<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let opts = parse_args()?;
    let apk = fs::read(&opts.apk)?;
    let entries = read_zip_entries(&apk)?;
    let mut counts: BTreeMap<ApiCall, u64> = BTreeMap::new();
    let mut profile = ApkProfile::default();
    let mut dex_count = 0usize;

    for entry in &entries {
        if entry.name == "AndroidManifest.xml" {
            let manifest = extract_zip_entry(&apk, entry)?;
            match parse_android_manifest(&manifest) {
                Ok(manifest) => profile.manifest = manifest,
                Err(err) => eprintln!("warning: failed to parse AndroidManifest.xml: {err}"),
            }
        } else if is_native_lib(&entry.name) {
            profile.native_libs.insert(entry.name.clone());
        } else if is_classes_dex(&entry.name) {
            dex_count += 1;
            let dex = extract_zip_entry(&apk, entry)?;
            let strings = extract_dex_strings(&dex)?;
            collect_domains(&strings, &mut profile);
            collect_dex_packages(&dex, &mut profile)?;
            let dex_calls = extract_dex_calls(&dex)?;
            for (api, count) in dex_calls {
                if opts.all || is_android_api(&api.class_name) {
                    *counts.entry(api).or_insert(0) += count;
                }
            }
        }
    }

    if dex_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "APK does not contain classes*.dex",
        ));
    }

    if opts.spec {
        print_edgerun_app_spec(&profile, &counts, opts.top);
    } else if opts.profile {
        print_profile(&profile, &counts, opts.top);
    } else if opts.summary {
        print_summary(&counts, opts.top);
    } else if opts.json {
        print_json(&counts);
    } else {
        for (api, count) in counts {
            println!(
                "{count}\t{}->{}({}){}",
                api.class_name,
                api.method_name,
                api.args.join(","),
                api.return_type
            );
        }
    }

    Ok(())
}

fn parse_args() -> io::Result<Options> {
    let mut apk = None;
    let mut all = false;
    let mut json = false;
    let mut summary = false;
    let mut profile = false;
    let mut spec = false;
    let mut top = 20usize;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--all" => all = true,
            "--json" => json = true,
            "--summary" => summary = true,
            "--profile" => profile = true,
            "--spec" => spec = true,
            "--top" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--top requires a number")
                })?;
                top = value.parse::<usize>().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid --top value: {value}"),
                    )
                })?;
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ if arg.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown option: {arg}"),
                ));
            }
            _ => {
                if apk.replace(arg).is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "expected one APK path",
                    ));
                }
            }
        }
    }

    let Some(apk) = apk else {
        print_usage();
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing APK path",
        ));
    };

    Ok(Options {
        apk,
        all,
        json,
        summary,
        profile,
        spec,
        top,
    })
}

fn print_usage() {
    eprintln!(
        "usage: edgerun-apk-api-calls [--all] [--json] [--summary] [--profile] [--spec] [--top N] <app.apk>\n\
         \n\
         Lists DEX invoke targets with count, class, method, argument types, and return type.\n\
         Default output includes Android platform APIs only. Use --all for every invoke target.\n\
         Use --summary to group APIs into top packages/classes/methods and capability signals.\n\
         Use --profile to add manifest permissions, components, domains, and native libs.\n\
         Use --spec to emit an Edgerun app replacement skeleton."
    );
}

fn is_classes_dex(name: &str) -> bool {
    if name == "classes.dex" {
        return true;
    }
    let Some(rest) = name
        .strip_prefix("classes")
        .and_then(|s| s.strip_suffix(".dex"))
    else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}

fn is_android_api(class_name: &str) -> bool {
    class_name.starts_with("android.")
        || class_name.starts_with("dalvik.")
        || class_name.starts_with("java.")
        || class_name.starts_with("javax.")
        || class_name.starts_with("org.apache.http.")
        || class_name.starts_with("org.json.")
        || class_name.starts_with("org.w3c.dom.")
        || class_name.starts_with("org.xml.sax.")
}

fn is_native_lib(name: &str) -> bool {
    name.starts_with("lib/") && name.ends_with(".so")
}

fn collect_domains(strings: &[String], profile: &mut ApkProfile) {
    for value in strings {
        for domain in domains_from_string(value) {
            profile.domains.insert(domain);
        }
        if value.contains("http://") || value.contains("https://") {
            profile.url_count += 1;
        }
    }
}

fn domains_from_string(value: &str) -> Vec<String> {
    let mut domains = Vec::new();
    for scheme in ["https://", "http://"] {
        let mut rest = value;
        while let Some(idx) = rest.find(scheme) {
            let after = &rest[idx + scheme.len()..];
            let host_end = after
                .find(|ch: char| {
                    matches!(ch, '/' | ':' | '?' | '#' | '"' | '\'' | '<' | '>' | '\\')
                })
                .unwrap_or(after.len());
            let host = after[..host_end].trim_matches('.');
            if looks_like_domain(host) {
                domains.push(host.to_ascii_lowercase());
            }
            rest = &after[host_end..];
        }
    }
    domains
}

fn looks_like_domain(host: &str) -> bool {
    if host.len() < 4 || !host.contains('.') || host.contains('/') {
        return false;
    }
    host.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        && host
            .rsplit('.')
            .next()
            .is_some_and(|tld| tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()))
}

fn read_zip_entries(data: &[u8]) -> io::Result<Vec<ZipEntry>> {
    let eocd = find_eocd(data)?;
    let total_entries = read_u16(data, eocd + 10)? as usize;
    let central_dir_size = read_u32(data, eocd + 12)? as usize;
    let central_dir_off = read_u32(data, eocd + 16)? as usize;
    checked_range(data, central_dir_off, central_dir_size)?;

    let mut pos = central_dir_off;
    let mut entries = Vec::with_capacity(total_entries);
    for _ in 0..total_entries {
        if read_u32(data, pos)? != 0x0201_4b50 {
            return Err(invalid("bad ZIP central directory header"));
        }
        let flags = read_u16(data, pos + 8)?;
        let compression = read_u16(data, pos + 10)?;
        let compressed_size = read_u32(data, pos + 20)? as usize;
        let uncompressed_size = read_u32(data, pos + 24)? as usize;
        let name_len = read_u16(data, pos + 28)? as usize;
        let extra_len = read_u16(data, pos + 30)? as usize;
        let comment_len = read_u16(data, pos + 32)? as usize;
        let local_header_offset = read_u32(data, pos + 42)? as usize;
        let name_start = pos + 46;
        let name = read_string(data, name_start, name_len)?;
        if flags & 0x0001 != 0 {
            return Err(invalid("encrypted APK entries are not supported"));
        }
        entries.push(ZipEntry {
            name,
            compression,
            compressed_size,
            uncompressed_size,
            local_header_offset,
        });
        pos = name_start
            .checked_add(name_len)
            .and_then(|p| p.checked_add(extra_len))
            .and_then(|p| p.checked_add(comment_len))
            .ok_or_else(|| invalid("ZIP central directory overflow"))?;
    }
    Ok(entries)
}

fn find_eocd(data: &[u8]) -> io::Result<usize> {
    let min = data.len().saturating_sub(22 + 65_535);
    for pos in (min..=data.len().saturating_sub(22)).rev() {
        if data.get(pos..pos + 4) == Some(&[0x50, 0x4b, 0x05, 0x06]) {
            return Ok(pos);
        }
    }
    Err(invalid("ZIP end of central directory not found"))
}

fn extract_zip_entry(data: &[u8], entry: &ZipEntry) -> io::Result<Vec<u8>> {
    let pos = entry.local_header_offset;
    if read_u32(data, pos)? != 0x0403_4b50 {
        return Err(invalid("bad ZIP local file header"));
    }
    let name_len = read_u16(data, pos + 26)? as usize;
    let extra_len = read_u16(data, pos + 28)? as usize;
    let data_start = pos
        .checked_add(30)
        .and_then(|p| p.checked_add(name_len))
        .and_then(|p| p.checked_add(extra_len))
        .ok_or_else(|| invalid("ZIP local file header overflow"))?;
    checked_range(data, data_start, entry.compressed_size)?;
    let compressed = &data[data_start..data_start + entry.compressed_size];
    match entry.compression {
        0 => {
            if compressed.len() != entry.uncompressed_size {
                return Err(invalid("stored ZIP entry size mismatch"));
            }
            Ok(compressed.to_vec())
        }
        8 => {
            let out = miniz_oxide::inflate::decompress_to_vec_with_limit(
                compressed,
                entry.uncompressed_size,
            )
            .map_err(|_| invalid("failed to inflate ZIP entry"))?;
            if out.len() != entry.uncompressed_size {
                return Err(invalid("inflated ZIP entry size mismatch"));
            }
            Ok(out)
        }
        method => Err(invalid(format!(
            "unsupported ZIP compression method {method} for {}",
            entry.name
        ))),
    }
}

fn extract_dex_calls(data: &[u8]) -> io::Result<BTreeMap<ApiCall, u64>> {
    let header = read_dex_header(data)?;
    let strings = read_string_ids(data, &header)?;
    let types = read_type_ids(data, &header, &strings)?;
    let protos = read_proto_ids(data, &header, &types)?;
    let methods = read_method_ids(data, &header, &strings, &types, &protos)?;
    let mut counts = BTreeMap::new();

    for class_idx in 0..header.class_defs_size {
        let class_def = header.class_defs_off + class_idx * 32;
        let class_data_off = read_u32(data, class_def + 24)? as usize;
        if class_data_off == 0 {
            continue;
        }
        for code_off in read_class_code_offsets(data, class_data_off)? {
            for method_idx in read_invoke_method_indices(data, code_off)? {
                if let Some(method) = methods.get(method_idx as usize) {
                    let api = ApiCall {
                        class_name: method.class_name.clone(),
                        method_name: method.method_name.clone(),
                        args: method.args.clone(),
                        return_type: method.return_type.clone(),
                    };
                    *counts.entry(api).or_insert(0) += 1;
                }
            }
        }
    }

    Ok(counts)
}

fn extract_dex_strings(data: &[u8]) -> io::Result<Vec<String>> {
    let header = read_dex_header(data)?;
    read_string_ids(data, &header)
}

fn collect_dex_packages(data: &[u8], profile: &mut ApkProfile) -> io::Result<()> {
    let header = read_dex_header(data)?;
    let strings = read_string_ids(data, &header)?;
    let types = read_type_ids(data, &header, &strings)?;
    checked_range(data, header.class_defs_off, header.class_defs_size * 32)?;
    for class_idx in 0..header.class_defs_size {
        let class_def = header.class_defs_off + class_idx * 32;
        let class_type_idx = read_u32(data, class_def)? as usize;
        if let Some(class_name) = types.get(class_type_idx) {
            let package = package_name(class_name);
            if !package.is_empty() {
                *profile.dex_packages.entry(package).or_insert(0) += 1;
            }
        }
    }
    Ok(())
}

fn parse_android_manifest(data: &[u8]) -> io::Result<ManifestProfile> {
    if data.len() < 8 || read_u16(data, 0)? != 0x0003 {
        return Err(invalid("AndroidManifest.xml is not binary XML"));
    }

    let mut strings = Vec::new();
    let mut manifest = ManifestProfile::default();
    let mut pos = 8usize;
    let mut current_component: Option<(String, ComponentProfile)> = None;
    let mut in_intent_filter = false;

    while pos + 8 <= data.len() {
        let chunk_type = read_u16(data, pos)?;
        let header_size = read_u16(data, pos + 2)? as usize;
        let chunk_size = read_u32(data, pos + 4)? as usize;
        if chunk_size < header_size || chunk_size == 0 {
            return Err(invalid("bad binary XML chunk size"));
        }

        match chunk_type {
            0x0001 => {
                strings = parse_string_pool(data, pos)?;
            }
            0x0102 => {
                let elem = parse_start_element(data, pos, &strings)?;
                match elem.name.as_str() {
                    "manifest" => {
                        manifest.package_name = elem.attr("package").map(str::to_string);
                    }
                    "uses-permission" | "uses-permission-sdk-23" => {
                        if let Some(name) = elem.attr("name").or_else(|| elem.attr("android:name"))
                        {
                            manifest.permissions.insert(name.to_string());
                        }
                    }
                    "activity" | "activity-alias" | "service" | "receiver" | "provider" => {
                        if let Some((kind, component)) = current_component.take() {
                            push_component(&mut manifest, &kind, component);
                        }
                        let name = elem
                            .attr("name")
                            .or_else(|| elem.attr("android:name"))
                            .unwrap_or("<unnamed>")
                            .to_string();
                        let exported = elem
                            .attr("exported")
                            .or_else(|| elem.attr("android:exported"))
                            .and_then(parse_bool);
                        current_component = Some((
                            elem.name.clone(),
                            ComponentProfile {
                                name,
                                exported,
                                ..ComponentProfile::default()
                            },
                        ));
                    }
                    "intent-filter" => in_intent_filter = true,
                    "action" if in_intent_filter => {
                        if let Some((_, component)) = current_component.as_mut() {
                            if let Some(name) =
                                elem.attr("name").or_else(|| elem.attr("android:name"))
                            {
                                component.actions.insert(name.to_string());
                            }
                        }
                    }
                    "category" if in_intent_filter => {
                        if let Some((_, component)) = current_component.as_mut() {
                            if let Some(name) =
                                elem.attr("name").or_else(|| elem.attr("android:name"))
                            {
                                component.categories.insert(name.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            0x0103 => {
                let name_idx = read_u32(data, pos + 20)? as usize;
                let name = strings.get(name_idx).map(String::as_str).unwrap_or("");
                match name {
                    "intent-filter" => in_intent_filter = false,
                    "activity" | "activity-alias" | "service" | "receiver" | "provider" => {
                        if let Some((kind, component)) = current_component.take() {
                            push_component(&mut manifest, &kind, component);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        pos = pos
            .checked_add(chunk_size)
            .ok_or_else(|| invalid("binary XML chunk overflow"))?;
    }

    if let Some((kind, component)) = current_component.take() {
        push_component(&mut manifest, &kind, component);
    }

    Ok(manifest)
}

#[derive(Clone, Debug)]
struct XmlElement {
    name: String,
    attrs: Vec<(String, String)>,
}

impl XmlElement {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

fn parse_string_pool(data: &[u8], off: usize) -> io::Result<Vec<String>> {
    let header_size = read_u16(data, off + 2)? as usize;
    let chunk_size = read_u32(data, off + 4)? as usize;
    let string_count = read_u32(data, off + 8)? as usize;
    let flags = read_u32(data, off + 16)?;
    let strings_start = read_u32(data, off + 20)? as usize;
    let is_utf8 = flags & 0x0000_0100 != 0;
    checked_range(data, off, chunk_size)?;
    checked_range(data, off + header_size, string_count * 4)?;

    let mut strings = Vec::with_capacity(string_count);
    for idx in 0..string_count {
        let string_off = read_u32(data, off + header_size + idx * 4)? as usize;
        let pos = off
            .checked_add(strings_start)
            .and_then(|p| p.checked_add(string_off))
            .ok_or_else(|| invalid("string pool offset overflow"))?;
        strings.push(if is_utf8 {
            read_utf8_pool_string(data, pos)?
        } else {
            read_utf16_pool_string(data, pos)?
        });
    }
    Ok(strings)
}

fn parse_start_element(data: &[u8], off: usize, strings: &[String]) -> io::Result<XmlElement> {
    let name_idx = read_u32(data, off + 20)? as usize;
    let attr_start = read_u16(data, off + 24)? as usize;
    let attr_size = read_u16(data, off + 26)? as usize;
    let attr_count = read_u16(data, off + 28)? as usize;
    let attrs_off = off
        .checked_add(16)
        .and_then(|p| p.checked_add(attr_start))
        .ok_or_else(|| invalid("XML attribute offset overflow"))?;
    checked_range(data, attrs_off, attr_count * attr_size)?;

    let mut attrs = Vec::new();
    for idx in 0..attr_count {
        let attr_off = attrs_off + idx * attr_size;
        let ns_idx = read_u32(data, attr_off)? as usize;
        let attr_name_idx = read_u32(data, attr_off + 4)? as usize;
        let raw_value_idx = read_u32(data, attr_off + 8)?;
        let value_type = *data
            .get(attr_off + 15)
            .ok_or_else(|| invalid("XML attribute type out of range"))?;
        let value_data = read_u32(data, attr_off + 16)?;

        let mut name = strings
            .get(attr_name_idx)
            .cloned()
            .unwrap_or_else(|| format!("#{attr_name_idx}"));
        if let Some(ns) = strings.get(ns_idx) {
            if ns == "http://schemas.android.com/apk/res/android" {
                name = format!("android:{name}");
            }
        }

        let value = if raw_value_idx != u32::MAX {
            strings
                .get(raw_value_idx as usize)
                .cloned()
                .unwrap_or_default()
        } else {
            format_typed_xml_value(value_type, value_data, strings)
        };
        attrs.push((name, value));
    }

    Ok(XmlElement {
        name: strings
            .get(name_idx)
            .cloned()
            .unwrap_or_else(|| format!("#{name_idx}")),
        attrs,
    })
}

fn read_utf8_pool_string(data: &[u8], mut pos: usize) -> io::Result<String> {
    let _utf16_len = read_pool_len8(data, &mut pos)?;
    let byte_len = read_pool_len8(data, &mut pos)?;
    checked_range(data, pos, byte_len + 1)?;
    Ok(String::from_utf8_lossy(&data[pos..pos + byte_len]).into_owned())
}

fn read_utf16_pool_string(data: &[u8], mut pos: usize) -> io::Result<String> {
    let char_len = read_pool_len16(data, &mut pos)?;
    checked_range(data, pos, char_len * 2 + 2)?;
    let mut units = Vec::with_capacity(char_len);
    for idx in 0..char_len {
        units.push(read_u16(data, pos + idx * 2)?);
    }
    Ok(String::from_utf16_lossy(&units))
}

fn read_pool_len8(data: &[u8], pos: &mut usize) -> io::Result<usize> {
    let first = *data
        .get(*pos)
        .ok_or_else(|| invalid("string pool length out of range"))?;
    *pos += 1;
    if first & 0x80 == 0 {
        Ok(first as usize)
    } else {
        let second = *data
            .get(*pos)
            .ok_or_else(|| invalid("string pool length out of range"))?;
        *pos += 1;
        Ok((((first & 0x7f) as usize) << 8) | second as usize)
    }
}

fn read_pool_len16(data: &[u8], pos: &mut usize) -> io::Result<usize> {
    let first = read_u16(data, *pos)?;
    *pos += 2;
    if first & 0x8000 == 0 {
        Ok(first as usize)
    } else {
        let second = read_u16(data, *pos)?;
        *pos += 2;
        Ok((((first & 0x7fff) as usize) << 16) | second as usize)
    }
}

fn format_typed_xml_value(value_type: u8, value_data: u32, strings: &[String]) -> String {
    match value_type {
        0x03 => strings
            .get(value_data as usize)
            .cloned()
            .unwrap_or_default(),
        0x10 | 0x11 => value_data.to_string(),
        0x12 => (value_data != 0).to_string(),
        0x01 => format!("@0x{value_data:08x}"),
        _ => format!("0x{value_data:08x}"),
    }
}

fn push_component(manifest: &mut ManifestProfile, kind: &str, component: ComponentProfile) {
    match kind {
        "activity" | "activity-alias" => manifest.activities.push(component),
        "service" => manifest.services.push(component),
        "receiver" => manifest.receivers.push(component),
        "provider" => manifest.providers.push(component),
        _ => {}
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn read_dex_header(data: &[u8]) -> io::Result<DexHeader> {
    if data.len() < 112 || data.get(0..4) != Some(b"dex\n") {
        return Err(invalid("not a DEX file"));
    }
    Ok(DexHeader {
        string_ids_size: read_u32(data, 56)? as usize,
        string_ids_off: read_u32(data, 60)? as usize,
        type_ids_size: read_u32(data, 64)? as usize,
        type_ids_off: read_u32(data, 68)? as usize,
        proto_ids_size: read_u32(data, 72)? as usize,
        proto_ids_off: read_u32(data, 76)? as usize,
        method_ids_size: read_u32(data, 88)? as usize,
        method_ids_off: read_u32(data, 92)? as usize,
        class_defs_size: read_u32(data, 96)? as usize,
        class_defs_off: read_u32(data, 100)? as usize,
    })
}

fn read_string_ids(data: &[u8], header: &DexHeader) -> io::Result<Vec<String>> {
    checked_range(data, header.string_ids_off, header.string_ids_size * 4)?;
    let mut strings = Vec::with_capacity(header.string_ids_size);
    for idx in 0..header.string_ids_size {
        let off = read_u32(data, header.string_ids_off + idx * 4)? as usize;
        strings.push(read_dex_string(data, off)?);
    }
    Ok(strings)
}

fn read_type_ids(data: &[u8], header: &DexHeader, strings: &[String]) -> io::Result<Vec<String>> {
    checked_range(data, header.type_ids_off, header.type_ids_size * 4)?;
    let mut types = Vec::with_capacity(header.type_ids_size);
    for idx in 0..header.type_ids_size {
        let string_idx = read_u32(data, header.type_ids_off + idx * 4)? as usize;
        let descriptor = strings
            .get(string_idx)
            .ok_or_else(|| invalid("DEX type string index out of range"))?;
        types.push(format_type(descriptor));
    }
    Ok(types)
}

fn read_proto_ids(
    data: &[u8],
    header: &DexHeader,
    types: &[String],
) -> io::Result<Vec<(String, Vec<String>)>> {
    checked_range(data, header.proto_ids_off, header.proto_ids_size * 12)?;
    let mut protos = Vec::with_capacity(header.proto_ids_size);
    for idx in 0..header.proto_ids_size {
        let pos = header.proto_ids_off + idx * 12;
        let return_type_idx = read_u32(data, pos + 4)? as usize;
        let parameters_off = read_u32(data, pos + 8)? as usize;
        let return_type = types
            .get(return_type_idx)
            .ok_or_else(|| invalid("DEX proto return type index out of range"))?
            .clone();
        let args = if parameters_off == 0 {
            Vec::new()
        } else {
            read_type_list(data, parameters_off, types)?
        };
        protos.push((return_type, args));
    }
    Ok(protos)
}

fn read_method_ids(
    data: &[u8],
    header: &DexHeader,
    strings: &[String],
    types: &[String],
    protos: &[(String, Vec<String>)],
) -> io::Result<Vec<MethodRef>> {
    checked_range(data, header.method_ids_off, header.method_ids_size * 8)?;
    let mut methods = Vec::with_capacity(header.method_ids_size);
    for idx in 0..header.method_ids_size {
        let pos = header.method_ids_off + idx * 8;
        let class_idx = read_u16(data, pos)? as usize;
        let proto_idx = read_u16(data, pos + 2)? as usize;
        let name_idx = read_u32(data, pos + 4)? as usize;
        let class_name = types
            .get(class_idx)
            .ok_or_else(|| invalid("DEX method class index out of range"))?
            .clone();
        let (return_type, args) = protos
            .get(proto_idx)
            .ok_or_else(|| invalid("DEX method proto index out of range"))?;
        let method_name = strings
            .get(name_idx)
            .ok_or_else(|| invalid("DEX method name index out of range"))?
            .clone();
        methods.push(MethodRef {
            class_name,
            method_name,
            args: args.clone(),
            return_type: return_type.clone(),
        });
    }
    Ok(methods)
}

fn read_type_list(data: &[u8], off: usize, types: &[String]) -> io::Result<Vec<String>> {
    let size = read_u32(data, off)? as usize;
    checked_range(data, off + 4, size * 2)?;
    let mut out = Vec::with_capacity(size);
    for idx in 0..size {
        let type_idx = read_u16(data, off + 4 + idx * 2)? as usize;
        out.push(
            types
                .get(type_idx)
                .ok_or_else(|| invalid("DEX type list index out of range"))?
                .clone(),
        );
    }
    Ok(out)
}

fn read_class_code_offsets(data: &[u8], class_data_off: usize) -> io::Result<Vec<usize>> {
    let mut pos = class_data_off;
    let static_fields = read_uleb128(data, &mut pos)?;
    let instance_fields = read_uleb128(data, &mut pos)?;
    let direct_methods = read_uleb128(data, &mut pos)?;
    let virtual_methods = read_uleb128(data, &mut pos)?;

    for _ in 0..static_fields + instance_fields {
        let _field_idx_diff = read_uleb128(data, &mut pos)?;
        let _access_flags = read_uleb128(data, &mut pos)?;
    }

    let mut code_offsets = Vec::new();
    for _ in 0..direct_methods + virtual_methods {
        let _method_idx_diff = read_uleb128(data, &mut pos)?;
        let _access_flags = read_uleb128(data, &mut pos)?;
        let code_off = read_uleb128(data, &mut pos)? as usize;
        if code_off != 0 {
            code_offsets.push(code_off);
        }
    }
    Ok(code_offsets)
}

fn read_invoke_method_indices(data: &[u8], code_off: usize) -> io::Result<Vec<u32>> {
    checked_range(data, code_off, 16)?;
    let insns_size = read_u32(data, code_off + 12)? as usize;
    let insns_off = code_off + 16;
    checked_range(data, insns_off, insns_size * 2)?;
    let mut cursor = 0usize;
    let mut out = Vec::new();
    while cursor < insns_size {
        let unit = read_u16(data, insns_off + cursor * 2)?;
        let opcode = (unit & 0x00ff) as u8;
        if matches!(opcode, 0x6e..=0x72 | 0x74..=0x78 | 0xfa | 0xfb) {
            if cursor + 1 < insns_size {
                out.push(read_u16(data, insns_off + (cursor + 1) * 2)? as u32);
            }
        }
        let Ok(width) = instruction_width(data, insns_off, insns_size, cursor) else {
            break;
        };
        cursor += width;
    }
    Ok(out)
}

fn instruction_width(
    data: &[u8],
    insns_off: usize,
    insns_size: usize,
    cursor: usize,
) -> io::Result<usize> {
    let unit = read_u16(data, insns_off + cursor * 2)?;
    let opcode = (unit & 0x00ff) as u8;
    let high = (unit >> 8) as u8;
    let width = match opcode {
        0x00 if high == 0x01 => {
            let size = payload_u16(data, insns_off, insns_size, cursor + 1)? as usize;
            4 + size * 2
        }
        0x00 if high == 0x02 => {
            let size = payload_u16(data, insns_off, insns_size, cursor + 1)? as usize;
            2 + size * 4
        }
        0x00 if high == 0x03 => {
            let elem_width = payload_u16(data, insns_off, insns_size, cursor + 1)? as usize;
            let size = payload_u32(data, insns_off, insns_size, cursor + 2)? as usize;
            4 + (elem_width * size).div_ceil(2)
        }
        0x00 => 1,
        0x01..=0x0f => 1,
        0x10..=0x12 => 1,
        0x13 => 2,
        0x14 => 3,
        0x15..=0x16 => 2,
        0x17 => 3,
        0x18 => 5,
        0x19..=0x1a => 2,
        0x1b => 3,
        0x1c => 2,
        0x1d..=0x1e => 1,
        0x1f..=0x20 => 2,
        0x21 => 1,
        0x22..=0x23 => 2,
        0x24..=0x26 => 3,
        0x27 => 1,
        0x28 => 1,
        0x29 => 2,
        0x2a => 3,
        0x2b..=0x2c => 3,
        0x2d..=0x31 => 2,
        0x32..=0x37 => 2,
        0x38..=0x3d => 2,
        0x44..=0x51 => 2,
        0x52..=0x5f => 2,
        0x60..=0x6d => 2,
        0x6e..=0x72 => 3,
        0x74..=0x78 => 3,
        0x7b..=0x8f => 1,
        0x90..=0xaf => 2,
        0xb0..=0xcf => 1,
        0xd0..=0xd7 => 2,
        0xd8..=0xe2 => 2,
        0xe3..=0xf9 => 1,
        0xfa..=0xfb => 4,
        0xfc..=0xfd => 3,
        0xfe..=0xff => 2,
        _ => 1,
    };
    if width == 0 || cursor + width > insns_size {
        return Err(invalid("DEX instruction exceeds code item"));
    }
    Ok(width)
}

fn payload_u16(data: &[u8], insns_off: usize, insns_size: usize, cursor: usize) -> io::Result<u16> {
    if cursor >= insns_size {
        return Err(invalid("DEX payload exceeds code item"));
    }
    read_u16(data, insns_off + cursor * 2)
}

fn payload_u32(data: &[u8], insns_off: usize, insns_size: usize, cursor: usize) -> io::Result<u32> {
    if cursor + 1 >= insns_size {
        return Err(invalid("DEX payload exceeds code item"));
    }
    read_u32(data, insns_off + cursor * 2)
}

fn read_dex_string(data: &[u8], off: usize) -> io::Result<String> {
    let mut pos = off;
    let _utf16_size = read_uleb128(data, &mut pos)?;
    let start = pos;
    while pos < data.len() && data[pos] != 0 {
        pos += 1;
    }
    if pos >= data.len() {
        return Err(invalid("unterminated DEX string"));
    }
    Ok(String::from_utf8_lossy(&data[start..pos]).into_owned())
}

fn format_type(descriptor: &str) -> String {
    let (ty, rest) = parse_type_descriptor(descriptor);
    if rest.is_empty() {
        ty
    } else {
        descriptor.to_string()
    }
}

fn parse_type_descriptor(input: &str) -> (String, &str) {
    let Some(first) = input.as_bytes().first().copied() else {
        return (String::new(), input);
    };
    match first {
        b'V' => ("void".to_string(), &input[1..]),
        b'Z' => ("boolean".to_string(), &input[1..]),
        b'B' => ("byte".to_string(), &input[1..]),
        b'S' => ("short".to_string(), &input[1..]),
        b'C' => ("char".to_string(), &input[1..]),
        b'I' => ("int".to_string(), &input[1..]),
        b'J' => ("long".to_string(), &input[1..]),
        b'F' => ("float".to_string(), &input[1..]),
        b'D' => ("double".to_string(), &input[1..]),
        b'L' => {
            let Some(end) = input.find(';') else {
                return (input.to_string(), "");
            };
            (input[1..end].replace('/', "."), &input[end + 1..])
        }
        b'[' => {
            let (inner, rest) = parse_type_descriptor(&input[1..]);
            (format!("{inner}[]"), rest)
        }
        _ => (input.to_string(), ""),
    }
}

fn read_uleb128(data: &[u8], pos: &mut usize) -> io::Result<u32> {
    let mut result = 0u32;
    for shift in (0..35).step_by(7) {
        let byte = *data
            .get(*pos)
            .ok_or_else(|| invalid("ULEB128 exceeds input"))?;
        *pos += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(invalid("ULEB128 is too large"))
}

fn print_json(counts: &BTreeMap<ApiCall, u64>) {
    println!("[");
    for (idx, (api, count)) in counts.iter().enumerate() {
        let comma = if idx + 1 == counts.len() { "" } else { "," };
        println!(
            "  {{\"count\":{},\"class\":\"{}\",\"method\":\"{}\",\"args\":[{}],\"returns\":\"{}\"}}{}",
            count,
            json_escape(&api.class_name),
            json_escape(&api.method_name),
            api.args
                .iter()
                .map(|arg| format!("\"{}\"", json_escape(arg)))
                .collect::<Vec<_>>()
                .join(","),
            json_escape(&api.return_type),
            comma
        );
    }
    println!("]");
}

fn print_summary(counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let mut total_call_sites = 0u64;
    let mut classes = BTreeSet::new();
    let mut packages: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut class_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut method_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut interesting_packages: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut interesting_class_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut interesting_method_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut signal_counts: BTreeMap<&'static str, SummaryCount> = BTreeMap::new();

    for (api, count) in counts {
        total_call_sites += count;
        classes.insert(api.class_name.as_str());

        add_summary_count(&mut packages, package_name(&api.class_name), *count);
        add_summary_count(&mut class_counts, api.class_name.clone(), *count);
        add_summary_count(&mut method_counts, method_key(api), *count);

        if is_interesting_api(api) {
            add_summary_count(
                &mut interesting_packages,
                package_name(&api.class_name),
                *count,
            );
            add_summary_count(
                &mut interesting_class_counts,
                api.class_name.clone(),
                *count,
            );
            add_summary_count(&mut interesting_method_counts, method_key(api), *count);
        }

        for signal in signals() {
            if matches_signal(api, signal) {
                add_summary_count_by_ref(&mut signal_counts, signal.name, *count);
            }
        }
    }

    println!("summary");
    println!("  unique_signatures: {}", counts.len());
    println!("  total_call_sites: {total_call_sites}");
    println!("  unique_classes: {}", classes.len());
    println!();

    print_ranked("capability_signals", &signal_counts, top);
    print_ranked("top_interesting_packages", &interesting_packages, top);
    print_ranked("top_interesting_classes", &interesting_class_counts, top);
    print_ranked("top_interesting_methods", &interesting_method_counts, top);
    print_ranked("top_packages", &packages, top);
    print_ranked("top_classes", &class_counts, top);
    print_ranked("top_methods", &method_counts, top);
}

fn print_profile(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    println!("profile");
    println!(
        "  package: {}",
        profile
            .manifest
            .package_name
            .as_deref()
            .unwrap_or("<unknown>")
    );
    println!("  unique_signatures: {}", counts.len());
    println!("  total_call_sites: {}", counts.values().sum::<u64>());
    println!();

    println!("permissions");
    for permission in profile.manifest.permissions.iter().take(top) {
        println!("  {permission}");
    }
    if profile.manifest.permissions.len() > top {
        println!("  ... {} more", profile.manifest.permissions.len() - top);
    }
    println!();

    print_components("activities", &profile.manifest.activities, top);
    print_components("services", &profile.manifest.services, top);
    print_components("receivers", &profile.manifest.receivers, top);
    print_components("providers", &profile.manifest.providers, top);

    println!("native_libs");
    println!("  count: {}", profile.native_libs.len());
    for lib in profile.native_libs.iter().take(top) {
        println!("  {lib}");
    }
    if profile.native_libs.len() > top {
        println!("  ... {} more", profile.native_libs.len() - top);
    }
    println!();

    println!("network_indicators");
    println!("  url_strings: {}", profile.url_count);
    println!("  domains: {}", profile.domains.len());
    for domain in profile.domains.iter().take(top) {
        println!("  {domain}");
    }
    if profile.domains.len() > top {
        println!("  ... {} more", profile.domains.len() - top);
    }
    println!();

    print_summary(counts, top);
    print_capability_candidates(profile, counts, top);
    print_likely_behaviors(profile, counts);
}

fn print_components(label: &str, components: &[ComponentProfile], top: usize) {
    println!("{label}");
    println!("  count: {}", components.len());
    let exported = components
        .iter()
        .filter(|component| component.exported == Some(true))
        .count();
    if exported > 0 {
        println!("  exported: {exported}");
    }
    for component in components.iter().take(top) {
        let exported = component
            .exported
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {} exported={exported}", component.name);
        for action in component.actions.iter().take(3) {
            println!("    action: {action}");
        }
        for category in component.categories.iter().take(2) {
            println!("    category: {category}");
        }
    }
    if components.len() > top {
        println!("  ... {} more", components.len() - top);
    }
    println!();
}

fn print_likely_behaviors(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) {
    let mut behaviors = BTreeSet::new();
    let permissions = &profile.manifest.permissions;

    if permissions.iter().any(|p| p.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        behaviors.insert("uses device location");
    }
    if permissions.iter().any(|p| p.contains("CAMERA"))
        || signal_call_sites(counts, "camera_media_capture") > 0
    {
        behaviors.insert("uses camera or media capture/playback APIs");
    }
    if permissions.iter().any(|p| p.contains("CONTACTS"))
        || signal_call_sites(counts, "contacts_calendar") > 0
    {
        behaviors.insert("touches contacts or calendar data");
    }
    if permissions.iter().any(|p| p.contains("BLUETOOTH"))
        || signal_call_sites(counts, "bluetooth_nearby") > 0
    {
        behaviors.insert("uses Bluetooth or nearby-device APIs");
    }
    if permissions.iter().any(|p| p.contains("SMS"))
        || signal_call_sites(counts, "sms_telephony") > 0
    {
        behaviors.insert("uses SMS, phone, or telephony APIs");
    }
    if permissions.iter().any(|p| p.contains("INTERNET"))
        || !profile.domains.is_empty()
        || signal_call_sites(counts, "network_web") > 0
    {
        behaviors.insert("talks to network services");
    }
    if permissions.iter().any(|p| p.contains("NOTIFICATION"))
        || signal_call_sites(counts, "notifications") > 0
    {
        behaviors.insert("posts or manages notifications");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        behaviors.insert("stores structured local state");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        behaviors.insert("runs background work or reacts to system events");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        behaviors.insert("uses cryptography, certificates, or key storage");
    }
    if !profile.native_libs.is_empty() {
        behaviors.insert("loads native code");
    }

    println!("likely_behavior");
    for behavior in behaviors {
        println!("  {behavior}");
    }
}

fn print_capability_candidates(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let permissions = &profile.manifest.permissions;
    println!("edgerun_capability_candidates");

    if permissions.iter().any(|p| p.contains("INTERNET"))
        || !profile.domains.is_empty()
        || signal_call_sites(counts, "network_web") > 0
    {
        let sample = profile
            .domains
            .iter()
            .take(top.min(5))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if sample.is_empty() {
            println!("  network: constrained outbound HTTP/TLS");
        } else {
            println!("  network: constrained outbound HTTP/TLS to {sample}");
        }
    }
    if permissions.iter().any(|p| p.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        let precise = permissions.iter().any(|p| p.contains("FINE_LOCATION"));
        println!(
            "  location: {}",
            if precise {
                "precise or coarse"
            } else {
                "coarse"
            }
        );
    }
    if permissions.iter().any(|p| p.contains("CAMERA"))
        || signal_call_sites(counts, "camera_media_capture") > 0
    {
        println!("  camera_media: explicit capture/playback sessions");
    }
    if permissions.iter().any(|p| p.contains("BLUETOOTH"))
        || signal_call_sites(counts, "bluetooth_nearby") > 0
    {
        println!("  bluetooth_nearby: scan/connect with device allowlist");
    }
    if permissions.iter().any(|p| p.contains("CONTACTS"))
        || signal_call_sites(counts, "contacts_calendar") > 0
    {
        println!("  contacts_calendar: selected records only");
    }
    if permissions.iter().any(|p| p.contains("NOTIFICATION"))
        || signal_call_sites(counts, "notifications") > 0
    {
        println!("  notifications: app-scoped notification channel");
    }
    if signal_call_sites(counts, "files_storage") > 0 {
        println!("  storage: app-private files plus explicit document grants");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        println!("  local_state: app-private structured store");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        println!("  background_tasks: scheduled jobs and declared event triggers");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        println!("  secrets_crypto: key store, signatures, certificates");
    }
    if !profile.native_libs.is_empty() {
        println!("  native_code: isolate or replace native library behavior");
    }
    println!();
}

fn signal_call_sites(counts: &BTreeMap<ApiCall, u64>, name: &str) -> u64 {
    let Some(signal) = signals().iter().find(|signal| signal.name == name) else {
        return 0;
    };
    counts
        .iter()
        .filter(|(api, _)| matches_signal(api, signal))
        .map(|(_, count)| *count)
        .sum()
}

#[derive(Clone, Copy, Debug, Default)]
struct SummaryCount {
    signatures: u64,
    call_sites: u64,
}

fn add_summary_count(map: &mut BTreeMap<String, SummaryCount>, key: String, call_sites: u64) {
    let entry = map.entry(key).or_default();
    entry.signatures += 1;
    entry.call_sites += call_sites;
}

fn add_summary_count_by_ref(
    map: &mut BTreeMap<&'static str, SummaryCount>,
    key: &'static str,
    call_sites: u64,
) {
    let entry = map.entry(key).or_default();
    entry.signatures += 1;
    entry.call_sites += call_sites;
}

fn print_ranked<K: AsRef<str> + Ord>(label: &str, map: &BTreeMap<K, SummaryCount>, top: usize) {
    println!("{label}");
    let mut rows: Vec<_> = map.iter().collect();
    rows.sort_by(|(left_key, left), (right_key, right)| {
        right
            .call_sites
            .cmp(&left.call_sites)
            .then_with(|| right.signatures.cmp(&left.signatures))
            .then_with(|| left_key.as_ref().cmp(right_key.as_ref()))
    });
    for (key, count) in rows.into_iter().take(top) {
        println!(
            "  {}\t{} call_sites\t{} signatures",
            key.as_ref(),
            count.call_sites,
            count.signatures
        );
    }
    println!();
}

fn package_name(class_name: &str) -> String {
    let mut parts = class_name.rsplitn(2, '.');
    let _class = parts.next();
    parts.next().unwrap_or(class_name).to_string()
}

fn method_key(api: &ApiCall) -> String {
    format!("{}->{}", api.class_name, api.method_name)
}

fn is_interesting_api(api: &ApiCall) -> bool {
    !is_boilerplate_class(&api.class_name) && !is_boilerplate_method(api)
}

fn is_boilerplate_class(class_name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "java.lang.",
        "java.util.",
        "kotlin.",
        "kotlinx.",
        "android.animation.",
        "android.graphics.",
        "android.text.",
        "android.util.",
        "android.view.",
        "android.widget.",
        "androidx.compose.",
    ];
    const EXACT: &[&str] = &[
        "android.os.Bundle",
        "android.os.Handler",
        "android.os.Looper",
        "android.os.Message",
        "android.os.Parcel",
        "android.os.Parcelable",
        "android.os.Parcelable$Creator",
        "android.os.SystemClock",
    ];
    PREFIXES.iter().any(|prefix| class_name.starts_with(prefix)) || EXACT.contains(&class_name)
}

fn is_boilerplate_method(api: &ApiCall) -> bool {
    matches!(
        api.method_name.as_str(),
        "<init>" | "equals" | "hashCode" | "toString"
    )
}

fn matches_signal(api: &ApiCall, signal: &Signal) -> bool {
    signal
        .prefixes
        .iter()
        .any(|prefix| api.class_name.starts_with(prefix))
        || signal
            .keywords
            .iter()
            .any(|keyword| api.class_name.contains(keyword) || api.method_name.contains(keyword))
}

fn signals() -> &'static [Signal] {
    &[
        Signal {
            name: "accounts_identity",
            prefixes: &["android.accounts.", "android.credentials."],
            keywords: &["Account", "Credential", "Auth", "Login", "SignIn"],
        },
        Signal {
            name: "activity_ui",
            prefixes: &[
                "android.app.",
                "android.view.",
                "android.widget.",
                "android.window.",
            ],
            keywords: &["Activity", "Dialog", "Fragment", "Window", "View"],
        },
        Signal {
            name: "bluetooth_nearby",
            prefixes: &["android.bluetooth.", "android.companion."],
            keywords: &["Bluetooth", "Ble", "CompanionDevice"],
        },
        Signal {
            name: "camera_media_capture",
            prefixes: &["android.hardware.camera", "android.media."],
            keywords: &["Camera", "MediaRecorder", "AudioRecord", "Microphone"],
        },
        Signal {
            name: "contacts_calendar",
            prefixes: &["android.provider.Contacts", "android.provider.Calendar"],
            keywords: &["Contacts", "Calendar"],
        },
        Signal {
            name: "crypto_security",
            prefixes: &["javax.crypto.", "java.security.", "android.security."],
            keywords: &["Cipher", "KeyStore", "Certificate", "Signature", "Crypto"],
        },
        Signal {
            name: "database_preferences",
            prefixes: &["android.database.", "android.preference."],
            keywords: &["SQLite", "SharedPreferences", "Preference"],
        },
        Signal {
            name: "files_storage",
            prefixes: &[
                "android.provider.Documents",
                "android.provider.MediaStore",
                "java.io.",
                "java.nio.file.",
            ],
            keywords: &["File", "Storage", "Document", "MediaStore"],
        },
        Signal {
            name: "location",
            prefixes: &["android.location."],
            keywords: &["Location", "Gps", "Gnss", "Geocoder"],
        },
        Signal {
            name: "network_web",
            prefixes: &[
                "android.net.",
                "android.webkit.",
                "java.net.",
                "javax.net.",
                "org.apache.http.",
            ],
            keywords: &["Socket", "Http", "Url", "Network", "WebView"],
        },
        Signal {
            name: "notifications",
            prefixes: &["android.app.Notification", "android.service.notification."],
            keywords: &["Notification"],
        },
        Signal {
            name: "package_intents",
            prefixes: &["android.content.", "android.content.pm."],
            keywords: &["Intent", "PackageManager", "BroadcastReceiver"],
        },
        Signal {
            name: "sensors",
            prefixes: &["android.hardware."],
            keywords: &["Sensor", "Vibrator", "Nfc", "Fingerprint", "Biometric"],
        },
        Signal {
            name: "sms_telephony",
            prefixes: &["android.telephony.", "android.provider.Telephony"],
            keywords: &["Sms", "Mms", "Telephony", "PhoneNumber"],
        },
        Signal {
            name: "work_background",
            prefixes: &["android.app.job.", "android.app.usage.", "android.os."],
            keywords: &[
                "JobScheduler",
                "AlarmManager",
                "WakeLock",
                "Work",
                "Service",
            ],
        },
    ]
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn read_string(data: &[u8], off: usize, len: usize) -> io::Result<String> {
    checked_range(data, off, len)?;
    Ok(String::from_utf8_lossy(&data[off..off + len]).into_owned())
}

fn read_u16(data: &[u8], off: usize) -> io::Result<u16> {
    checked_range(data, off, 2)?;
    Ok(u16::from_le_bytes([data[off], data[off + 1]]))
}

fn read_u32(data: &[u8], off: usize) -> io::Result<u32> {
    checked_range(data, off, 4)?;
    Ok(u32::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
    ]))
}

fn checked_range(data: &[u8], off: usize, len: usize) -> io::Result<()> {
    if off.checked_add(len).is_some_and(|end| end <= data.len()) {
        Ok(())
    } else {
        Err(invalid("input range is out of bounds"))
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
