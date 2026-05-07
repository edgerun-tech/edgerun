#![cfg(feature = "std")]

use edgerun_standards::{
    analyze_tftp, contract_unit, graph_hash as contract_graph_hash, sha256, unit_hash,
    wat_for_clause, wat_for_definition, ContractUnit, Finding, Severity, TraceItem, CLAUSES,
    CONTRACT_GRAPH, CONTRACT_UNITS, DEFINITIONS, PROGRAM_ID, SHA1_FIPS180_FIXED_ABI_WAT,
    SHA256_FIPS180_FIXED_ABI_WAT, TFTP_RFC1350_FIXED_ABI_WAT, UDP_RFC768_FIXED_ABI_WAT,
};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT_FROM_CRATE: &str = "../..";

struct Case {
    id: &'static str,
    file: &'static str,
    description: &'static str,
    expect_exit: i32,
    expect: &'static [(&'static str, Severity)],
}

const CASES: &[Case] = &[
    Case {
        id: "valid-ack",
        file: "valid-ack.hex",
        description: "Valid TFTP ACK for block 1.",
        expect_exit: 0,
        expect: &[
            ("tftp-rfc1350-opcode-0001", Severity::Pass),
            ("tftp-rfc1350-ack-length-0001", Severity::Pass),
            ("tftp-rfc1350-data-length-0001", Severity::Pass),
        ],
    },
    Case {
        id: "invalid-short",
        file: "invalid-short.hex",
        description: "TFTP message is shorter than the two-octet opcode.",
        expect_exit: 2,
        expect: &[("tftp-message-definition", Severity::Reject)],
    },
    Case {
        id: "invalid-ack-length",
        file: "invalid-ack-length.hex",
        description: "TFTP ACK opcode with an extra trailing octet.",
        expect_exit: 2,
        expect: &[
            ("tftp-rfc1350-opcode-0001", Severity::Pass),
            ("tftp-rfc1350-ack-length-0001", Severity::Reject),
            ("tftp-rfc1350-data-length-0001", Severity::Pass),
        ],
    },
    Case {
        id: "invalid-data-length",
        file: "invalid-data-length.hex",
        description: "TFTP DATA opcode without a two-octet block number.",
        expect_exit: 2,
        expect: &[
            ("tftp-rfc1350-opcode-0001", Severity::Pass),
            ("tftp-rfc1350-ack-length-0001", Severity::Pass),
            ("tftp-rfc1350-data-length-0001", Severity::Reject),
        ],
    },
    Case {
        id: "invalid-opcode",
        file: "invalid-opcode.hex",
        description: "TFTP message with an unknown opcode.",
        expect_exit: 2,
        expect: &[
            ("tftp-rfc1350-opcode-0001", Severity::Reject),
            ("tftp-rfc1350-ack-length-0001", Severity::Pass),
            ("tftp-rfc1350-data-length-0001", Severity::Pass),
        ],
    },
];

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("validate") => cmd_validate(),
        Some("hash") => cmd_hash(args.get(1).map(String::as_str)),
        Some("compile") => cmd_compile(
            args.get(1).map(String::as_str),
            args.iter().any(|a| a == "--quiet"),
        ),
        Some("compile-abi") => cmd_compile_abi(args.get(1).map(String::as_str)),
        Some("compile-graph") => cmd_compile_graph(args.get(1).map(String::as_str)),
        Some("compile-hashed-graph") => cmd_compile_hashed_graph(args.get(1).map(String::as_str)),
        Some("compile-hashes") => cmd_compile_hashes(args.get(1).map(String::as_str)),
        Some("components") => cmd_components(args.get(1).map(String::as_str)),
        Some("definitions") => cmd_definitions(args.get(1).map(String::as_str)),
        Some("clauses") => cmd_clauses(args.get(1).map(String::as_str)),
        Some("units") => cmd_units(args.get(1).map(String::as_str)),
        Some("graph") => cmd_graph(args.get(1).map(String::as_str)),
        Some("program") => cmd_program(args.get(1).map(String::as_str)),
        Some("run") => cmd_run(&args[1..]),
        Some("check") => cmd_check(args.get(1).map(String::as_str)),
        Some("list") => cmd_list(),
        _ => {
            eprintln!("usage: standards <validate|hash|compile|compile-abi|compile-graph|compile-hashed-graph|compile-hashes|components|definitions|clauses|units|graph|program|run|check|list>");
            1
        }
    };
    std::process::exit(code);
}

fn cmd_validate() -> i32 {
    let root = root();
    let required = [
        "standards/programs/tftp.toml",
        "standards/definitions/tftp.toml",
        "standards/clauses/tftp.toml",
        "standards/corpus/tftp/cases.toml",
    ];
    for item in required {
        if !root.join(item).exists() {
            eprintln!("missing required catalog file: {item}");
            return 1;
        }
    }
    println!("standards catalog valid");
    println!(
        "  protocols: {}",
        count_files(root.join("standards/protocols"), "toml")
    );
    println!("  definitions: {}", DEFINITIONS.len());
    println!("  clauses: {}", CLAUSES.len());
    println!("  contract_units: {}", CONTRACT_UNITS.len());
    println!("  programs: 1");
    0
}

fn cmd_hash(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    println!("{}", hex(&program_hash()));
    0
}

fn cmd_compile(program: Option<&str>, quiet: bool) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    let root = root();
    let out_dir = root.join("standards/build/wasm").join(PROGRAM_ID);
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("compile failed: {err}");
        return 1;
    }
    if let Err(err) = write_contract_units(&root) {
        eprintln!("compile failed: {err}");
        return 1;
    }

    let mut components = Vec::new();
    for definition in DEFINITIONS {
        let component_id = format!("{}-definition", definition.id);
        if !graph_has_node(&component_id) {
            continue;
        }
        let Some(wat) = wat_for_definition(definition.id) else {
            eprintln!("compile failed: no WAT for {}", definition.id);
            return 1;
        };
        match compile_component(
            &out_dir,
            &component_id,
            "definition",
            definition.id,
            "minimum_length",
            &[],
            wat,
        ) {
            Ok(component) => components.push(component),
            Err(err) => {
                eprintln!("compile failed: {err}");
                return 1;
            }
        }
    }
    for clause in CLAUSES {
        if !graph_has_node(clause.id) {
            continue;
        }
        let Some(wat) = wat_for_clause(clause.id) else {
            eprintln!("compile failed: no WAT for {}", clause.id);
            return 1;
        };
        match compile_component(
            &out_dir,
            clause.id,
            "clause",
            clause.subject,
            "check",
            clause.inputs,
            wat,
        ) {
            Ok(component) => components.push(component),
            Err(err) => {
                eprintln!("compile failed: {err}");
                return 1;
            }
        }
    }

    let graph_preimage = components
        .iter()
        .map(|component| component.json.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let component_graph_hash = sha256(graph_preimage.as_bytes());
    let program_hash = program_hash();
    let assembly = assembly_json(
        &components,
        &hex(&component_graph_hash),
        &hex(&program_hash),
    );
    let assembly_path = out_dir.join("program.json");
    if let Err(err) = fs::write(&assembly_path, assembly) {
        eprintln!("compile failed: {err}");
        return 1;
    }

    if !quiet {
        println!(
            "{{\n  \"assembly\": \"standards/build/wasm/{}/program.json\",\n  \"program_sha256\": \"{}\",\n  \"component_graph_sha256\": \"{}\"\n}}",
            PROGRAM_ID,
            hex(&program_hash),
            hex(&component_graph_hash)
        );
    }
    0
}

fn cmd_compile_abi(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    let Ok(tftp) = compile_fixed_abi_unit("tftp-rfc1350", TFTP_RFC1350_FIXED_ABI_WAT) else {
        return 1;
    };
    println!("{{");
    println!("  \"standard\": \"RFC1350\",");
    println!("  \"unit\": \"tftp-rfc1350\",");
    println!("  \"abi_version\": 1,");
    println!("  \"wasm\": \"{}\",", tftp.path);
    println!("  \"sha256\": \"{}\"", tftp.sha256);
    println!("}}");
    0
}

fn cmd_compile_graph(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    let Ok(udp) = compile_fixed_abi_unit("udp-rfc768", UDP_RFC768_FIXED_ABI_WAT) else {
        return 1;
    };
    let Ok(tftp) = compile_fixed_abi_unit("tftp-rfc1350", TFTP_RFC1350_FIXED_ABI_WAT) else {
        return 1;
    };

    let root = root();
    let out_dir = root.join("standards/build/wasm/udp-tftp-fixed-abi");
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("compile-graph failed: {err}");
        return 1;
    }
    let preimage = format!(
        "edgerun-rfc-graph/v1\nid=udp-tftp\nabi=1\nnode=udp-rfc768@{}\nnode=tftp-rfc1350@{}\nedge=udp-rfc768.payload->tftp-rfc1350.input_bytes\n",
        udp.sha256, tftp.sha256
    );
    let graph_hash = hex(&sha256(preimage.as_bytes()));
    let graph = format!(
        "{{\n  \"id\": \"udp-tftp\",\n  \"abi_version\": 1,\n  \"sha256\": \"{}\",\n  \"canonical_preimage\": \"{}\",\n  \"nodes\": [\n    {{\"id\":\"udp-rfc768\",\"standard\":\"RFC768\",\"wasm\":\"{}\",\"sha256\":\"{}\"}},\n    {{\"id\":\"tftp-rfc1350\",\"standard\":\"RFC1350\",\"wasm\":\"{}\",\"sha256\":\"{}\"}}\n  ],\n  \"edges\": [\n    {{\"from\":\"udp-rfc768.payload\",\"to\":\"tftp-rfc1350.input_bytes\",\"frame\":\"INPUT_DATAGRAM\",\"predicate\":\"udp.length == input_len && (source_port == 69 || destination_port == 69)\"}}\n  ]\n}}\n",
        graph_hash,
        json_escape(&preimage),
        udp.path,
        udp.sha256,
        tftp.path,
        tftp.sha256
    );
    let graph_path = out_dir.join("graph.json");
    if let Err(err) = fs::write(&graph_path, graph) {
        eprintln!("compile-graph failed: {err}");
        return 1;
    }

    println!("{{");
    println!("  \"graph\": \"standards/build/wasm/udp-tftp-fixed-abi/graph.json\",");
    println!("  \"sha256\": \"{}\",", graph_hash);
    println!("  \"nodes\": [\"udp-rfc768\", \"tftp-rfc1350\"]");
    println!("}}");
    0
}

fn cmd_compile_hashed_graph(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    let Ok(udp) = compile_fixed_abi_unit("udp-rfc768", UDP_RFC768_FIXED_ABI_WAT) else {
        return 1;
    };
    let Ok(tftp) = compile_fixed_abi_unit("tftp-rfc1350", TFTP_RFC1350_FIXED_ABI_WAT) else {
        return 1;
    };
    let Ok(sha256_unit) = compile_fixed_abi_unit("sha256-fips180", SHA256_FIPS180_FIXED_ABI_WAT)
    else {
        return 1;
    };

    let root = root();
    let out_dir = root.join("standards/build/wasm/udp-tftp-hashed-fixed-abi");
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("compile-hashed-graph failed: {err}");
        return 1;
    }
    let preimage = format!(
        "edgerun-rfc-graph/v1\nid=udp-tftp-hashed\nabi=1\nnode=udp-rfc768@{}\nnode=tftp-rfc1350@{}\nnode=sha256-fips180@{}\nedge=input.bytes->sha256-fips180.datagram\nedge=input.bytes->udp-rfc768\nedge=udp-rfc768.payload->tftp-rfc1350.input_bytes\nedge=udp-rfc768.payload->sha256-fips180.payload\n",
        udp.sha256, tftp.sha256, sha256_unit.sha256
    );
    let graph_hash = hex(&sha256(preimage.as_bytes()));
    let graph = format!(
        "{{\n  \"id\": \"udp-tftp-hashed\",\n  \"abi_version\": 1,\n  \"sha256\": \"{}\",\n  \"canonical_preimage\": \"{}\",\n  \"nodes\": [\n    {{\"id\":\"udp-rfc768\",\"standard\":\"RFC768\",\"wasm\":\"{}\",\"sha256\":\"{}\"}},\n    {{\"id\":\"tftp-rfc1350\",\"standard\":\"RFC1350\",\"wasm\":\"{}\",\"sha256\":\"{}\"}},\n    {{\"id\":\"sha256-fips180\",\"standard\":\"FIPS180-4\",\"wasm\":\"{}\",\"sha256\":\"{}\"}}\n  ],\n  \"edges\": [\n    {{\"from\":\"input.bytes\",\"to\":\"sha256-fips180.datagram\",\"frame\":\"INPUT_BYTES\"}},\n    {{\"from\":\"input.bytes\",\"to\":\"udp-rfc768\",\"frame\":\"INPUT_DATAGRAM\"}},\n    {{\"from\":\"udp-rfc768.payload\",\"to\":\"tftp-rfc1350.input_bytes\",\"frame\":\"INPUT_DATAGRAM\",\"predicate\":\"udp.length == input_len && (source_port == 69 || destination_port == 69)\"}},\n    {{\"from\":\"udp-rfc768.payload\",\"to\":\"sha256-fips180.payload\",\"frame\":\"INPUT_BYTES\",\"predicate\":\"udp.length == input_len\"}}\n  ]\n}}\n",
        graph_hash,
        json_escape(&preimage),
        udp.path,
        udp.sha256,
        tftp.path,
        tftp.sha256,
        sha256_unit.path,
        sha256_unit.sha256
    );
    let graph_path = out_dir.join("graph.json");
    if let Err(err) = fs::write(&graph_path, graph) {
        eprintln!("compile-hashed-graph failed: {err}");
        return 1;
    }

    println!("{{");
    println!("  \"graph\": \"standards/build/wasm/udp-tftp-hashed-fixed-abi/graph.json\",");
    println!("  \"sha256\": \"{}\",", graph_hash);
    println!("  \"nodes\": [\"udp-rfc768\", \"tftp-rfc1350\", \"sha256-fips180\"]");
    println!("}}");
    0
}

fn cmd_compile_hashes(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    let Ok(sha1) = compile_fixed_abi_unit("sha1-fips180", SHA1_FIPS180_FIXED_ABI_WAT) else {
        return 1;
    };
    let Ok(sha256) = compile_fixed_abi_unit("sha256-fips180", SHA256_FIPS180_FIXED_ABI_WAT) else {
        return 1;
    };
    println!("{{");
    println!("  \"units\": [");
    println!("    {{\"id\":\"sha1-fips180\",\"standard\":\"FIPS180-4\",\"wasm\":\"{}\",\"sha256\":\"{}\"}},", sha1.path, sha1.sha256);
    println!("    {{\"id\":\"sha256-fips180\",\"standard\":\"FIPS180-4\",\"wasm\":\"{}\",\"sha256\":\"{}\"}}", sha256.path, sha256.sha256);
    println!("  ]");
    println!("}}");
    0
}

struct FixedAbiUnit {
    path: String,
    sha256: String,
}

fn compile_fixed_abi_unit(unit: &str, wat: &str) -> Result<FixedAbiUnit, ()> {
    let root = root();
    let out_dir = root.join("standards/build/wasm").join(unit);
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("compile-abi failed: {err}");
        return Err(());
    }

    let wat_path = out_dir.join(format!("{unit}.wat"));
    let wasm_path = out_dir.join(format!("{unit}.wasm"));
    if let Err(err) = fs::write(&wat_path, wat) {
        eprintln!("compile-abi failed: {err}");
        return Err(());
    }
    if let Err(err) = run_tool(
        "wat2wasm",
        &[
            wat_path.as_os_str(),
            OsStr::new("-o"),
            wasm_path.as_os_str(),
        ],
    ) {
        eprintln!("compile-abi failed: {err}");
        return Err(());
    }
    if tool_exists("wasm-tools") {
        if let Err(err) = run_tool(
            "wasm-tools",
            &[OsStr::new("validate"), wasm_path.as_os_str()],
        ) {
            eprintln!("compile-abi failed: {err}");
            return Err(());
        }
    }

    let wasm = match fs::read(&wasm_path) {
        Ok(wasm) => wasm,
        Err(err) => {
            eprintln!("compile-abi failed: {err}");
            return Err(());
        }
    };
    Ok(FixedAbiUnit {
        path: format!("standards/build/wasm/{unit}/{unit}.wasm"),
        sha256: hex(&sha256(&wasm)),
    })
}

fn cmd_components(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    if cmd_compile(program, true) != 0 {
        return 1;
    }
    for definition in DEFINITIONS {
        let id = format!("{}-definition", definition.id);
        if !graph_has_node(&id) {
            continue;
        }
        println!("{id:32} definition standards/build/wasm/{PROGRAM_ID}/{id}.wasm");
    }
    for clause in CLAUSES {
        if !graph_has_node(clause.id) {
            continue;
        }
        println!(
            "{:32} clause     standards/build/wasm/{}/{}.wasm",
            clause.id, PROGRAM_ID, clause.id
        );
    }
    0
}

fn cmd_definitions(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    for definition in DEFINITIONS {
        println!("{}", definition.id);
        println!("  kind: definition");
        println!("  exports: {}", definition.exports.join(", "));
    }
    0
}

fn cmd_clauses(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    for clause in CLAUSES {
        println!("{}", clause.id);
        println!("  subject: {}", clause.subject);
        println!("  standard: {} section {}", clause.standard, clause.section);
        println!("  keyword: {}", clause.keyword);
        println!("  expr: {}", clause.expr);
        println!("  inputs: {}", clause.inputs.join(", "));
    }
    0
}

fn cmd_units(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    for unit in CONTRACT_UNITS {
        println!("{} {}", hex(&unit_hash(&unit)), unit.id);
        println!("  kind: {}", unit.kind.as_str());
        println!("  standard: {} section {}", unit.standard, unit.section);
        println!("  imports: {}", unit.imports.join(", "));
        println!("  exports: {}", unit.exports.join(", "));
        println!("  wasm_export: {}", unit.wasm_export);
    }
    0
}

fn cmd_graph(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    println!(
        "{} ({})",
        CONTRACT_GRAPH.id,
        hex(&contract_graph_hash(&CONTRACT_GRAPH))
    );
    println!("  profile: {}", CONTRACT_GRAPH.profile);
    println!("  nodes:");
    for node in CONTRACT_GRAPH.nodes {
        if let Some(unit) = contract_unit(node) {
            println!("    {} {}", hex(&unit_hash(unit)), unit.id);
        } else {
            println!("    <missing> {node}");
        }
    }
    println!("  edges:");
    for edge in CONTRACT_GRAPH.edges {
        println!(
            "    {} -> {} [{}] when {} via {}",
            edge.from, edge.to, edge.interface, edge.rule, edge.binding
        );
    }
    0
}

fn cmd_program(program: Option<&str>) -> i32 {
    if !program_ok(program) {
        return 1;
    }
    println!("TFTP MUST-only program ({PROGRAM_ID})");
    println!(
        "  graph_hash: {}",
        hex(&contract_graph_hash(&CONTRACT_GRAPH))
    );
    println!("  profile: must-only");
    println!("  definitions: tftp-message");
    println!(
        "  clauses: {}",
        CLAUSES
            .iter()
            .map(|clause| clause.id)
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  edges:");
    println!("    input.bytes -> tftp-message-definition [bytes -> tftp-message]");
    0
}

fn cmd_run(args: &[String]) -> i32 {
    let Some(program) = args.first().map(String::as_str) else {
        eprintln!("missing program");
        return 1;
    };
    if !program_ok(Some(program)) {
        return 1;
    }
    let mut hex_arg = None;
    let mut index = 1;
    while index < args.len() {
        if args[index] == "--hex" {
            hex_arg = args.get(index + 1).map(String::as_str);
            index += 1;
        }
        index += 1;
    }
    let Some(hex_arg) = hex_arg else {
        eprintln!("missing --hex");
        return 1;
    };
    let data = match decode_hex(hex_arg) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("invalid hex input: {err}");
            return 1;
        }
    };
    let report = analyze_tftp(&data);
    print_report(&report, "rust-core");
    report.exit_code()
}

fn cmd_check(program: Option<&str>) -> i32 {
    if cmd_validate() != 0 || cmd_compile(program, true) != 0 {
        return 1;
    }

    let mut failed = false;
    println!("{{");
    println!("  \"program\": \"{PROGRAM_ID}\",");
    println!("  \"ok\": true,");
    println!("  \"cases\": [");
    for (case_index, case) in CASES.iter().enumerate() {
        let path = root().join("standards/corpus/tftp").join(case.file);
        let data = match fs::read_to_string(&path).map(|s| decode_hex(s.trim())) {
            Ok(Ok(data)) => data,
            Ok(Err(err)) => {
                eprintln!("{}: {err}", path.display());
                return 1;
            }
            Err(err) => {
                eprintln!("{}: {err}", path.display());
                return 1;
            }
        };
        let report = analyze_tftp(&data);
        let signature_ok = expected_signature_matches(&report, case.expect);
        let exit_ok = report.exit_code() == case.expect_exit;
        let ok = signature_ok && exit_ok;
        failed |= !ok;
        println!("    {{");
        println!("      \"case\": \"{}\",", case.id);
        println!("      \"path\": \"standards/corpus/tftp/{}\",", case.file);
        println!(
            "      \"description\": \"{}\",",
            json_escape(case.description)
        );
        println!("      \"expected_exit\": {},", case.expect_exit);
        println!("      \"rust_exit\": {},", report.exit_code());
        println!(
            "      \"expected_signature_match\": {},",
            ok_bool(signature_ok)
        );
        println!("      \"ok\": {},", ok_bool(ok));
        print_signature_field("expected_findings", case.expect, 6, true);
        print_report_signature_field("rust_findings", &report, 6, false);
        println!(
            "    }}{}",
            if case_index + 1 == CASES.len() {
                ""
            } else {
                ","
            }
        );
    }
    println!("  ],");
    println!("  \"assembly\": \"standards/build/wasm/{PROGRAM_ID}/program.json\"");
    println!("}}");
    if failed {
        1
    } else {
        0
    }
}

fn cmd_list() -> i32 {
    for id in [
        "ethernet", "arp", "ipv4", "icmpv4", "udp", "tcp", "tftp", "dns", "dhcpv4", "dhcpv6",
        "tls", "http", "hpack", "quic", "qpack", "ipv6", "icmpv6", "nfc",
    ] {
        println!("{id}");
    }
    0
}

fn graph_has_node(id: &str) -> bool {
    CONTRACT_GRAPH.nodes.iter().any(|node| *node == id)
}

struct ComponentOut {
    json: String,
}

fn compile_component(
    out_dir: &Path,
    id: &str,
    kind: &str,
    subject: &str,
    entrypoint: &str,
    inputs: &[&str],
    wat: &str,
) -> io::Result<ComponentOut> {
    let wat_path = out_dir.join(format!("{id}.wat"));
    let wasm_path = out_dir.join(format!("{id}.wasm"));
    fs::write(&wat_path, wat)?;
    run_tool(
        "wat2wasm",
        &[
            wat_path.as_os_str(),
            OsStr::new("-o"),
            wasm_path.as_os_str(),
        ],
    )?;
    if tool_exists("wasm-tools") {
        run_tool(
            "wasm-tools",
            &[OsStr::new("validate"), wasm_path.as_os_str()],
        )?;
    }
    let wasm = fs::read(&wasm_path)?;
    let digest = hex(&sha256(&wasm));
    let Some(unit) = contract_unit(id) else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("missing contract unit for {id}"),
        ));
    };
    let unit_digest = hex(&unit_hash(unit));
    let manifest_path = root()
        .join("standards/components/manifests")
        .join(format!("{id}.toml"));
    fs::write(
        manifest_path,
        format!(
            "id = \"{id}\"\nname = \"{id}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\ncontract_unit = \"{id}\"\ncontract_unit_sha256 = \"{unit_digest}\"\nwat = \"standards/build/wasm/{PROGRAM_ID}/{id}.wat\"\nwasm = \"standards/build/wasm/{PROGRAM_ID}/{id}.wasm\"\nsha256 = \"{digest}\"\nentrypoint = \"{entrypoint}\"\ndeterministic = true\ninputs = [{}]\noutputs = [\"i32\"]\ndepends_on = []\n",
            quote_list(inputs)
        ),
    )?;
    Ok(ComponentOut {
        json: format!(
            "{{\"id\":\"{id}\",\"kind\":\"{kind}\",\"contract_unit\":\"{id}\",\"contract_unit_sha256\":\"{unit_digest}\",\"manifest\":\"standards/components/manifests/{id}.toml\",\"wasm\":\"standards/build/wasm/{PROGRAM_ID}/{id}.wasm\",\"sha256\":\"{digest}\",\"subject\":\"{subject}\",\"entrypoint\":\"{entrypoint}\",\"inputs\":[{}]}}",
            quote_list(inputs)
        ),
    })
}

fn assembly_json(components: &[ComponentOut], graph_hash: &str, program_hash: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"program\": {{\"id\":\"{}\",\"profile\":\"must-only\"}},\n",
        PROGRAM_ID
    ));
    out.push_str(&format!(
        "  \"contract_graph\": {},\n",
        graph_json(&CONTRACT_GRAPH)
    ));
    out.push_str("  \"policy\": {\"id\":\"must-only\"},\n");
    out.push_str("  \"compiler\": {\"crate\":\"edgerun-standards\",\"language\":\"rust\",\"no_std_core\":true},\n");
    out.push_str("  \"components\": [\n");
    for (index, component) in components.iter().enumerate() {
        out.push_str("    ");
        out.push_str(&component.json);
        out.push_str(if index + 1 == components.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    out.push_str("  ],\n");
    out.push_str(&format!(
        "  \"component_graph_sha256\": \"{graph_hash}\",\n"
    ));
    out.push_str(&format!("  \"program_sha256\": \"{program_hash}\"\n"));
    out.push_str("}\n");
    out
}

fn write_contract_units(root: &Path) -> io::Result<()> {
    let unit_dir = root.join("standards/build/units");
    fs::create_dir_all(&unit_dir)?;
    for unit in CONTRACT_UNITS {
        let digest = hex(&unit_hash(&unit));
        fs::write(
            unit_dir.join(format!("{digest}.json")),
            unit_json(&unit, &digest),
        )?;
    }
    let graph_digest = hex(&contract_graph_hash(&CONTRACT_GRAPH));
    fs::write(
        root.join("standards/build/units")
            .join(format!("{graph_digest}.graph.json")),
        graph_json(&CONTRACT_GRAPH),
    )?;
    Ok(())
}

fn unit_json(unit: &ContractUnit, digest: &str) -> String {
    format!(
        "{{\"id\":\"{}\",\"sha256\":\"{}\",\"kind\":\"{}\",\"standard\":\"{}\",\"section\":\"{}\",\"normative_text\":\"{}\",\"ir\":\"{}\",\"imports\":[{}],\"exports\":[{}],\"required_capabilities\":[{}],\"wasm_export\":\"{}\"}}\n",
        json_escape(unit.id),
        digest,
        unit.kind.as_str(),
        json_escape(unit.standard),
        json_escape(unit.section),
        json_escape(unit.normative_text),
        json_escape(unit.ir),
        quote_list(unit.imports),
        quote_list(unit.exports),
        quote_list(unit.required_capabilities),
        json_escape(unit.wasm_export)
    )
}

fn graph_json(graph: &edgerun_standards::ContractGraph) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\"id\":\"{}\",\"profile\":\"{}\",\"sha256\":\"{}\",\"nodes\":[",
        graph.id,
        graph.profile,
        hex(&contract_graph_hash(graph))
    ));
    for (index, node) in graph.nodes.iter().enumerate() {
        let unit_hash = contract_unit(node)
            .map(unit_hash)
            .map(|hash| hex(&hash))
            .unwrap_or_default();
        out.push_str(&format!(
            "{{\"id\":\"{}\",\"sha256\":\"{}\"}}{}",
            node,
            unit_hash,
            if index + 1 == graph.nodes.len() {
                ""
            } else {
                ","
            }
        ));
    }
    out.push_str("],\"edges\":[");
    for (index, edge) in graph.edges.iter().enumerate() {
        out.push_str(&format!(
            "{{\"from\":\"{}\",\"to\":\"{}\",\"interface\":\"{}\",\"rule\":\"{}\",\"binding\":\"{}\"}}{}",
            json_escape(edge.from),
            json_escape(edge.to),
            json_escape(edge.interface),
            json_escape(edge.rule),
            json_escape(edge.binding),
            if index + 1 == graph.edges.len() { "" } else { "," }
        ));
    }
    out.push_str("]}");
    out
}

fn print_report(report: &edgerun_standards::Report<'_>, engine: &str) {
    println!("{{");
    println!("  \"program\": \"{PROGRAM_ID}\",");
    println!("  \"engine\": \"{engine}\",");
    println!("  \"program_sha256\": \"{}\",", hex(&program_hash()));
    println!("  \"trace\": [");
    for index in 0..report.trace_count() {
        let comma = if index + 1 == report.trace_count() {
            ""
        } else {
            ","
        };
        match report.trace()[index] {
            Some(TraceItem::Input { bytes }) => {
                println!("    {{\"seq\":{index},\"kind\":\"input\",\"protocol\":\"{PROGRAM_ID}\",\"bytes_hex\":\"{}\"}}{comma}", hex(bytes));
            }
            Some(TraceItem::Udp(udp)) => {
                println!("    {{\"seq\":{index},\"kind\":\"parse\",\"protocol\":\"udp\",\"source_port\":{},\"destination_port\":{},\"length\":{},\"checksum\":{},\"payload_hex\":\"{}\"}}{comma}", udp.source_port, udp.destination_port, udp.length, udp.checksum, hex(udp.payload));
            }
            Some(TraceItem::Tftp(tftp)) => {
                println!("    {{\"seq\":{index},\"kind\":\"parse\",\"protocol\":\"tftp\",\"opcode\":{},\"payload_hex\":\"{}\"}}{comma}", tftp.opcode, hex(tftp.payload));
            }
            None => {}
        }
    }
    println!("  ],");
    println!("  \"findings\": [");
    for index in 0..report.finding_count() {
        if let Some(finding) = report.findings()[index] {
            print_finding(finding, 4, index + 1 == report.finding_count());
        }
    }
    println!("  ]");
    println!("}}");
}

fn print_finding(finding: Finding, indent: usize, last: bool) {
    let pad = " ".repeat(indent);
    println!("{pad}{{");
    println!("{pad}  \"requirement\": \"{}\",", finding.requirement);
    println!("{pad}  \"severity\": \"{}\",", finding.severity.as_str());
    println!("{pad}  \"message\": \"{}\"", json_escape(finding.message));
    println!("{pad}}}{}", if last { "" } else { "," });
}

fn expected_signature_matches(
    report: &edgerun_standards::Report<'_>,
    expected: &[(&str, Severity)],
) -> bool {
    if report.finding_count() != expected.len() {
        return false;
    }
    for (index, (id, severity)) in expected.iter().enumerate() {
        let Some(finding) = report.findings()[index] else {
            return false;
        };
        if finding.requirement != *id || finding.severity != *severity {
            return false;
        }
    }
    true
}

fn print_signature_field(name: &str, values: &[(&str, Severity)], indent: usize, comma: bool) {
    let pad = " ".repeat(indent);
    println!("{pad}\"{name}\": [");
    for (index, (id, severity)) in values.iter().enumerate() {
        println!(
            "{pad}  [\"{}\", \"{}\"]{}",
            id,
            severity.as_str(),
            if index + 1 == values.len() { "" } else { "," }
        );
    }
    println!("{pad}]{}", if comma { "," } else { "" });
}

fn print_report_signature_field(
    name: &str,
    report: &edgerun_standards::Report<'_>,
    indent: usize,
    comma: bool,
) {
    let pad = " ".repeat(indent);
    println!("{pad}\"{name}\": [");
    for index in 0..report.finding_count() {
        if let Some(finding) = report.findings()[index] {
            println!(
                "{pad}  [\"{}\", \"{}\"]{}",
                finding.requirement,
                finding.severity.as_str(),
                if index + 1 == report.finding_count() {
                    ""
                } else {
                    ","
                }
            );
        }
    }
    println!("{pad}]{}", if comma { "," } else { "" });
}

fn program_ok(program: Option<&str>) -> bool {
    if program == Some(PROGRAM_ID) {
        return true;
    }
    eprintln!("unknown program: {}", program.unwrap_or(""));
    false
}

fn program_hash() -> [u8; 32] {
    contract_graph_hash(&CONTRACT_GRAPH)
}

fn root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join(ROOT_FROM_CRATE)
}

fn count_files(path: PathBuf, ext: &str) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(OsStr::to_str) == Some(ext))
        .count()
}

fn run_tool(tool: &str, args: &[&OsStr]) -> io::Result<()> {
    let status = Command::new(tool).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("{tool} failed"),
        ))
    }
}

fn tool_exists(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn decode_hex(input: &str) -> Result<Vec<u8>, &'static str> {
    let compact: Vec<u8> = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if compact.len() % 2 != 0 {
        return Err("odd number of hex digits");
    }
    let mut out = Vec::with_capacity(compact.len() / 2);
    let mut index = 0;
    while index < compact.len() {
        let hi = hex_value(compact[index]).ok_or("invalid hex digit")?;
        let lo = hex_value(compact[index + 1]).ok_or("invalid hex digit")?;
        out.push((hi << 4) | lo);
        index += 2;
    }
    Ok(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn quote_list(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("\"{item}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn json_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn ok_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
