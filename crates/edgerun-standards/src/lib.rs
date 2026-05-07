#![no_std]

pub const PROGRAM_ID: &str = "tftp-must-program";
pub const TFTP_PORT: u16 = 69;
pub const TFTP_OP_DATA: u16 = 3;
pub const TFTP_OP_ACK: u16 = 4;

pub const UDP_LENGTH: &str = "udp-rfc768-length-0001";
pub const TFTP_OPCODE: &str = "tftp-rfc1350-opcode-0001";
pub const TFTP_ACK_LENGTH: &str = "tftp-rfc1350-ack-length-0001";
pub const TFTP_DATA_LENGTH: &str = "tftp-rfc1350-data-length-0001";

pub const UDP_DEFINITION_UNIT: &str = "udp-datagram-definition";
pub const TFTP_DEFINITION_UNIT: &str = "tftp-message-definition";

pub const CLAUSES: [Clause; 4] = [
    Clause {
        id: UDP_LENGTH,
        subject: "udp-datagram",
        standard: "RFC768",
        section: "Format",
        keyword: "MUST",
        expr: "length >= 8 && length == payload_len + 8",
        inputs: &["length", "payload_len"],
        message: "UDP length does not match header plus payload length.",
    },
    Clause {
        id: TFTP_OPCODE,
        subject: "tftp-message",
        standard: "RFC1350",
        section: "5",
        keyword: "MUST",
        expr: "byte_len >= 2 && opcode in [1, 2, 3, 4, 5, 6]",
        inputs: &["byte_len", "opcode"],
        message: "TFTP opcode is missing or unknown.",
    },
    Clause {
        id: TFTP_ACK_LENGTH,
        subject: "tftp-message",
        standard: "RFC1350",
        section: "5",
        keyword: "MUST",
        expr: "opcode != 4 || byte_len == 4",
        inputs: &["byte_len", "opcode"],
        message: "TFTP ACK packets must be exactly four octets.",
    },
    Clause {
        id: TFTP_DATA_LENGTH,
        subject: "tftp-message",
        standard: "RFC1350",
        section: "5",
        keyword: "MUST",
        expr: "opcode != 3 || byte_len >= 4",
        inputs: &["byte_len", "opcode"],
        message: "TFTP DATA packets must include a two-octet block number.",
    },
];

pub const CONTRACT_UNITS: [ContractUnit; 6] = [
    ContractUnit {
        id: UDP_DEFINITION_UNIT,
        kind: UnitKind::Definition,
        standard: "RFC768",
        section: "Format",
        normative_text: "UDP header fields are source port, destination port, length, checksum, and data.",
        ir: "definition udp-datagram { u16_be source_port @0; u16_be destination_port @2; u16_be length @4; u16_be checksum @6; bytes payload @8.. }",
        imports: &["input.bytes"],
        exports: &[
            "udp.source_port:u16",
            "udp.destination_port:u16",
            "udp.length:u16",
            "udp.checksum:u16",
            "udp.payload:bytes",
        ],
        required_capabilities: &[],
        wasm_export: "minimum_length",
    },
    ContractUnit {
        id: TFTP_DEFINITION_UNIT,
        kind: UnitKind::Definition,
        standard: "RFC1350",
        section: "5",
        normative_text: "TFTP packets begin with a two-octet opcode.",
        ir: "definition tftp-message { u16_be opcode @0; bytes payload @2.. }",
        imports: &["input.bytes"],
        exports: &["tftp.opcode:u16", "tftp.payload:bytes", "tftp.byte_len:u32"],
        required_capabilities: &[],
        wasm_export: "minimum_length",
    },
    ContractUnit {
        id: UDP_LENGTH,
        kind: UnitKind::Clause,
        standard: "RFC768",
        section: "Format",
        normative_text: "The UDP length field covers the header and data octets.",
        ir: "clause udp-rfc768-length-0001 { require length >= 8 && length == payload_len + 8 }",
        imports: &["udp.length:u16", "udp.payload_len:u32"],
        exports: &["finding:udp-rfc768-length-0001"],
        required_capabilities: &[],
        wasm_export: "check",
    },
    ContractUnit {
        id: TFTP_OPCODE,
        kind: UnitKind::Clause,
        standard: "RFC1350",
        section: "5",
        normative_text: "A TFTP message starts with a two-octet opcode identifying the operation.",
        ir: "clause tftp-rfc1350-opcode-0001 { require byte_len >= 2 && opcode in [1, 2, 3, 4, 5, 6] }",
        imports: &["tftp.byte_len:u32", "tftp.opcode:u16"],
        exports: &["finding:tftp-rfc1350-opcode-0001"],
        required_capabilities: &[],
        wasm_export: "check",
    },
    ContractUnit {
        id: TFTP_ACK_LENGTH,
        kind: UnitKind::Clause,
        standard: "RFC1350",
        section: "5",
        normative_text: "An ACK packet consists of opcode 4 followed by a two-octet block number.",
        ir: "clause tftp-rfc1350-ack-length-0001 { require opcode != 4 || byte_len == 4 }",
        imports: &["tftp.byte_len:u32", "tftp.opcode:u16"],
        exports: &["finding:tftp-rfc1350-ack-length-0001"],
        required_capabilities: &[],
        wasm_export: "check",
    },
    ContractUnit {
        id: TFTP_DATA_LENGTH,
        kind: UnitKind::Clause,
        standard: "RFC1350",
        section: "5",
        normative_text: "A DATA packet consists of opcode 3, a two-octet block number, and zero or more data octets.",
        ir: "clause tftp-rfc1350-data-length-0001 { require opcode != 3 || byte_len >= 4 }",
        imports: &["tftp.byte_len:u32", "tftp.opcode:u16"],
        exports: &["finding:tftp-rfc1350-data-length-0001"],
        required_capabilities: &[],
        wasm_export: "check",
    },
];

pub const CONTRACT_GRAPH: ContractGraph = ContractGraph {
    id: PROGRAM_ID,
    profile: "must-only",
    nodes: &[
        TFTP_DEFINITION_UNIT,
        TFTP_OPCODE,
        TFTP_ACK_LENGTH,
        TFTP_DATA_LENGTH,
    ],
    edges: &[
        GraphEdge {
            from: "input.bytes",
            to: TFTP_DEFINITION_UNIT,
            interface: "bytes -> tftp-message",
            rule: "always",
            binding: "caller-provided-bytes",
        },
        GraphEdge {
            from: TFTP_DEFINITION_UNIT,
            to: TFTP_OPCODE,
            interface: "tftp.byte_len + tftp.opcode -> clause inputs",
            rule: "tftp-message exists",
            binding: "field-export",
        },
        GraphEdge {
            from: TFTP_DEFINITION_UNIT,
            to: TFTP_ACK_LENGTH,
            interface: "tftp.byte_len + tftp.opcode -> clause inputs",
            rule: "tftp-message exists",
            binding: "field-export",
        },
        GraphEdge {
            from: TFTP_DEFINITION_UNIT,
            to: TFTP_DATA_LENGTH,
            interface: "tftp.byte_len + tftp.opcode -> clause inputs",
            rule: "tftp-message exists",
            binding: "field-export",
        },
    ],
};

pub const DEFINITIONS: [Definition; 2] = [
    Definition {
        id: "udp-datagram",
        exports: &[
            "minimum_length()",
            "source_port_at(ptr)",
            "destination_port_at(ptr)",
            "length_at(ptr)",
            "checksum_at(ptr)",
            "payload_offset(ptr)",
            "payload_len(byte_len)",
        ],
    },
    Definition {
        id: "tftp-message",
        exports: &[
            "minimum_length()",
            "opcode_at(ptr)",
            "payload_offset(ptr)",
            "payload_len(byte_len)",
        ],
    },
];

#[derive(Clone, Copy)]
pub struct Clause {
    pub id: &'static str,
    pub subject: &'static str,
    pub standard: &'static str,
    pub section: &'static str,
    pub keyword: &'static str,
    pub expr: &'static str,
    pub inputs: &'static [&'static str],
    pub message: &'static str,
}

#[derive(Clone, Copy)]
pub struct Definition {
    pub id: &'static str,
    pub exports: &'static [&'static str],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Definition,
    Clause,
    Adapter,
    Binding,
}

impl UnitKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Clause => "clause",
            Self::Adapter => "adapter",
            Self::Binding => "binding",
        }
    }
}

#[derive(Clone, Copy)]
pub struct ContractUnit {
    pub id: &'static str,
    pub kind: UnitKind,
    pub standard: &'static str,
    pub section: &'static str,
    pub normative_text: &'static str,
    pub ir: &'static str,
    pub imports: &'static [&'static str],
    pub exports: &'static [&'static str],
    pub required_capabilities: &'static [&'static str],
    pub wasm_export: &'static str,
}

#[derive(Clone, Copy)]
pub struct GraphEdge {
    pub from: &'static str,
    pub to: &'static str,
    pub interface: &'static str,
    pub rule: &'static str,
    pub binding: &'static str,
}

#[derive(Clone, Copy)]
pub struct ContractGraph {
    pub id: &'static str,
    pub profile: &'static str,
    pub nodes: &'static [&'static str],
    pub edges: &'static [GraphEdge],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Pass,
    Reject,
    Skip,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Reject => "reject",
            Self::Skip => "skip",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Finding {
    pub requirement: &'static str,
    pub severity: Severity,
    pub message: &'static str,
}

#[derive(Clone, Copy)]
pub struct UdpDatagram<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct TftpMessage<'a> {
    pub opcode: u16,
    pub payload: &'a [u8],
    pub byte_len: usize,
}

#[derive(Clone, Copy)]
pub enum TraceItem<'a> {
    Input { bytes: &'a [u8] },
    Udp(UdpDatagram<'a>),
    Tftp(TftpMessage<'a>),
}

pub struct Report<'a> {
    findings: [Option<Finding>; 4],
    finding_count: usize,
    trace: [Option<TraceItem<'a>>; 3],
    trace_count: usize,
}

impl<'a> Report<'a> {
    pub const fn empty() -> Self {
        Self {
            findings: [None, None, None, None],
            finding_count: 0,
            trace: [None, None, None],
            trace_count: 0,
        }
    }

    pub fn findings(&self) -> &[Option<Finding>; 4] {
        &self.findings
    }

    pub fn finding_count(&self) -> usize {
        self.finding_count
    }

    pub fn trace(&self) -> &[Option<TraceItem<'a>>; 3] {
        &self.trace
    }

    pub fn trace_count(&self) -> usize {
        self.trace_count
    }

    pub fn exit_code(&self) -> i32 {
        let mut index = 0;
        while index < self.finding_count {
            if let Some(finding) = self.findings[index] {
                if finding.severity == Severity::Reject {
                    return 2;
                }
            }
            index += 1;
        }
        0
    }

    fn push_finding(&mut self, finding: Finding) {
        if self.finding_count < self.findings.len() {
            self.findings[self.finding_count] = Some(finding);
            self.finding_count += 1;
        }
    }

    fn push_trace(&mut self, item: TraceItem<'a>) {
        if self.trace_count < self.trace.len() {
            self.trace[self.trace_count] = Some(item);
            self.trace_count += 1;
        }
    }
}

pub fn parse_udp_datagram(data: &[u8]) -> Option<UdpDatagram<'_>> {
    if data.len() < 8 {
        return None;
    }
    Some(UdpDatagram {
        source_port: u16_be(data, 0),
        destination_port: u16_be(data, 2),
        length: u16_be(data, 4),
        checksum: u16_be(data, 6),
        payload: &data[8..],
    })
}

pub fn parse_tftp_message(data: &[u8]) -> Option<TftpMessage<'_>> {
    if data.len() < 2 {
        return None;
    }
    Some(TftpMessage {
        opcode: u16_be(data, 0),
        payload: &data[2..],
        byte_len: data.len(),
    })
}

pub fn analyze_udp_tftp(data: &[u8]) -> Report<'_> {
    analyze_tftp(data)
}

pub fn analyze_tftp(data: &[u8]) -> Report<'_> {
    let mut report = Report::empty();
    report.push_trace(TraceItem::Input { bytes: data });

    let Some(tftp) = parse_tftp_message(data) else {
        report.push_finding(Finding {
            requirement: "tftp-message-definition",
            severity: Severity::Reject,
            message: "TFTP payload is shorter than the 2-octet opcode.",
        });
        return report;
    };

    report.push_trace(TraceItem::Tftp(tftp));
    let opcode_ok = tftp.byte_len >= 2 && (1..=6).contains(&tftp.opcode);
    report.push_finding(Finding {
        requirement: TFTP_OPCODE,
        severity: if opcode_ok {
            Severity::Pass
        } else {
            Severity::Reject
        },
        message: if opcode_ok {
            "TFTP opcode is known for the selected profile."
        } else {
            "TFTP opcode is missing or unknown."
        },
    });

    let ack_ok = tftp.opcode != TFTP_OP_ACK || tftp.byte_len == 4;
    report.push_finding(Finding {
        requirement: TFTP_ACK_LENGTH,
        severity: if ack_ok {
            Severity::Pass
        } else {
            Severity::Reject
        },
        message: if ack_ok {
            "TFTP ACK length requirement is satisfied or not applicable."
        } else {
            "TFTP ACK packets must be exactly four octets."
        },
    });

    let data_ok = tftp.opcode != TFTP_OP_DATA || tftp.byte_len >= 4;
    report.push_finding(Finding {
        requirement: TFTP_DATA_LENGTH,
        severity: if data_ok {
            Severity::Pass
        } else {
            Severity::Reject
        },
        message: if data_ok {
            "TFTP DATA length requirement is satisfied or not applicable."
        } else {
            "TFTP DATA packets must include a two-octet block number."
        },
    });

    report
}

pub fn wat_for_definition(definition_id: &str) -> Option<&'static str> {
    match definition_id {
        "udp-datagram" => Some(UDP_DEFINITION_WAT),
        "tftp-message" => Some(TFTP_DEFINITION_WAT),
        _ => None,
    }
}

pub fn wat_for_clause(clause_id: &str) -> Option<&'static str> {
    match clause_id {
        UDP_LENGTH => Some(UDP_LENGTH_WAT),
        TFTP_OPCODE => Some(TFTP_OPCODE_WAT),
        TFTP_ACK_LENGTH => Some(TFTP_ACK_LENGTH_WAT),
        TFTP_DATA_LENGTH => Some(TFTP_DATA_LENGTH_WAT),
        _ => None,
    }
}

pub fn contract_unit(id: &str) -> Option<&'static ContractUnit> {
    let mut index = 0;
    while index < CONTRACT_UNITS.len() {
        if str_eq(CONTRACT_UNITS[index].id, id) {
            return Some(&CONTRACT_UNITS[index]);
        }
        index += 1;
    }
    None
}

pub fn unit_hash(unit: &ContractUnit) -> [u8; 32] {
    let mut hasher = Sha256State::new();
    hasher.update(b"edgerun-contract-unit/v1\n");
    hash_field(&mut hasher, "id", unit.id);
    hash_field(&mut hasher, "kind", unit.kind.as_str());
    hash_field(&mut hasher, "standard", unit.standard);
    hash_field(&mut hasher, "section", unit.section);
    hash_field(&mut hasher, "normative_text", unit.normative_text);
    hash_field(&mut hasher, "ir", unit.ir);
    hash_list(&mut hasher, "imports", unit.imports);
    hash_list(&mut hasher, "exports", unit.exports);
    hash_list(
        &mut hasher,
        "required_capabilities",
        unit.required_capabilities,
    );
    hash_field(&mut hasher, "wasm_export", unit.wasm_export);
    hasher.finish()
}

pub fn graph_hash(graph: &ContractGraph) -> [u8; 32] {
    let mut hasher = Sha256State::new();
    hasher.update(b"edgerun-contract-graph/v1\n");
    hash_field(&mut hasher, "id", graph.id);
    hash_field(&mut hasher, "profile", graph.profile);
    hasher.update(b"nodes=[");
    let mut index = 0;
    while index < graph.nodes.len() {
        if let Some(unit) = contract_unit(graph.nodes[index]) {
            hasher.update(graph.nodes[index].as_bytes());
            hasher.update(b"@");
            hasher.update_hex(&unit_hash(unit));
            hasher.update(b";");
        }
        index += 1;
    }
    hasher.update(b"]\n");
    hasher.update(b"edges=[");
    let mut edge_index = 0;
    while edge_index < graph.edges.len() {
        let edge = graph.edges[edge_index];
        hasher.update(edge.from.as_bytes());
        hasher.update(b"->");
        hasher.update(edge.to.as_bytes());
        hasher.update(b"|");
        hasher.update(edge.interface.as_bytes());
        hasher.update(b"|");
        hasher.update(edge.rule.as_bytes());
        hasher.update(b"|");
        hasher.update(edge.binding.as_bytes());
        hasher.update(b";");
        edge_index += 1;
    }
    hasher.update(b"]\n");
    hasher.finish()
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h = H0;
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut chunk = [0u8; 64];
    let mut offset = 0;
    while offset + 64 <= data.len() {
        chunk.copy_from_slice(&data[offset..offset + 64]);
        sha256_compress(&mut h, &chunk, &K);
        offset += 64;
    }

    let rem = &data[offset..];
    chunk = [0u8; 64];
    let mut i = 0;
    while i < rem.len() {
        chunk[i] = rem[i];
        i += 1;
    }
    chunk[rem.len()] = 0x80;
    if rem.len() >= 56 {
        sha256_compress(&mut h, &chunk, &K);
        chunk = [0u8; 64];
    }
    chunk[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sha256_compress(&mut h, &chunk, &K);

    let mut out = [0u8; 32];
    let mut j = 0;
    while j < h.len() {
        out[j * 4..j * 4 + 4].copy_from_slice(&h[j].to_be_bytes());
        j += 1;
    }
    out
}

struct Sha256State {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    byte_len: u64,
}

impl Sha256State {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            byte_len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.byte_len = self.byte_len.wrapping_add(data.len() as u64);
        if self.buffer_len > 0 {
            let take = min_usize(64 - self.buffer_len, data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            data = &data[take..];
            if self.buffer_len == 64 {
                sha256_compress(&mut self.state, &self.buffer, &Self::K);
                self.buffer_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            sha256_compress(&mut self.state, &block, &Self::K);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffer_len = data.len();
        }
    }

    fn update_hex(&mut self, bytes: &[u8]) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut index = 0;
        while index < bytes.len() {
            let pair = [
                HEX[(bytes[index] >> 4) as usize],
                HEX[(bytes[index] & 0x0f) as usize],
            ];
            self.update(&pair);
            index += 1;
        }
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_len = self.byte_len.wrapping_mul(8);
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            let mut index = self.buffer_len;
            while index < 64 {
                self.buffer[index] = 0;
                index += 1;
            }
            sha256_compress(&mut self.state, &self.buffer, &Self::K);
            self.buffer = [0; 64];
            self.buffer_len = 0;
        }
        let mut index = self.buffer_len;
        while index < 56 {
            self.buffer[index] = 0;
            index += 1;
        }
        self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
        sha256_compress(&mut self.state, &self.buffer, &Self::K);
        let mut out = [0u8; 32];
        let mut state_index = 0;
        while state_index < self.state.len() {
            out[state_index * 4..state_index * 4 + 4]
                .copy_from_slice(&self.state[state_index].to_be_bytes());
            state_index += 1;
        }
        out
    }
}

fn hash_field(hasher: &mut Sha256State, key: &str, value: &str) {
    hasher.update(key.as_bytes());
    hasher.update(b"=");
    hasher.update(value.as_bytes());
    hasher.update(b"\n");
}

fn hash_list(hasher: &mut Sha256State, key: &str, values: &[&str]) {
    hasher.update(key.as_bytes());
    hasher.update(b"=[");
    let mut index = 0;
    while index < values.len() {
        hasher.update(values[index].as_bytes());
        hasher.update(b";");
        index += 1;
    }
    hasher.update(b"]\n");
}

fn str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn min_usize(left: usize, right: usize) -> usize {
    if left < right {
        left
    } else {
        right
    }
}

fn sha256_compress(h: &mut [u32; 8], chunk: &[u8; 64], k: &[u32; 64]) {
    let mut w = [0u32; 64];
    let mut i = 0;
    while i < 16 {
        w[i] = u32::from_be_bytes([
            chunk[i * 4],
            chunk[i * 4 + 1],
            chunk[i * 4 + 2],
            chunk[i * 4 + 3],
        ]);
        i += 1;
    }
    while i < 64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
        i += 1;
    }

    let mut a = h[0];
    let mut b = h[1];
    let mut c = h[2];
    let mut d = h[3];
    let mut e = h[4];
    let mut f = h[5];
    let mut g = h[6];
    let mut hh = h[7];
    i = 0;
    while i < 64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(k[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
        i += 1;
    }

    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(b);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
    h[5] = h[5].wrapping_add(f);
    h[6] = h[6].wrapping_add(g);
    h[7] = h[7].wrapping_add(hh);
}

const fn u16_be(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16) << 8) | data[offset + 1] as u16
}

pub const UDP_DEFINITION_WAT: &str = r#";; generated from udp-datagram
(module
  (memory (export "memory") 1)
  (func (export "minimum_length") (result i32) (i32.const 8))
  (func (export "source_port_at") (param $ptr i32) (result i32)
    (i32.or (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))
  (func (export "destination_port_at") (param $ptr i32) (result i32)
    (i32.or (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.const 3)))))
  (func (export "length_at") (param $ptr i32) (result i32)
    (i32.or (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.const 5)))))
  (func (export "checksum_at") (param $ptr i32) (result i32)
    (i32.or (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.const 7)))))
  (func (export "payload_offset") (param $ptr i32) (result i32)
    (i32.add (local.get $ptr) (i32.const 8)))
  (func (export "payload_len") (param $byte_len i32) (result i32)
    (i32.sub (local.get $byte_len) (i32.const 8)))
)
"#;

pub const TFTP_DEFINITION_WAT: &str = r#";; generated from tftp-message
(module
  (memory (export "memory") 1)
  (func (export "minimum_length") (result i32) (i32.const 2))
  (func (export "opcode_at") (param $ptr i32) (result i32)
    (i32.or (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
            (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))
  (func (export "payload_offset") (param $ptr i32) (result i32)
    (i32.add (local.get $ptr) (i32.const 2)))
  (func (export "payload_len") (param $byte_len i32) (result i32)
    (i32.sub (local.get $byte_len) (i32.const 2)))
)
"#;

pub const UDP_LENGTH_WAT: &str = r#";; generated from udp-rfc768-length-0001
;; expr: length >= 8 && length == payload_len + 8
(module
  (func (export "check") (param $length i32) (param $payload_len i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $length) (i32.const 8))
      (i32.eq (local.get $length) (i32.add (local.get $payload_len) (i32.const 8)))))
)
"#;

pub const TFTP_OPCODE_WAT: &str = r#";; generated from tftp-rfc1350-opcode-0001
;; expr: byte_len >= 2 && opcode in [1, 2, 3, 4, 5, 6]
(module
  (func (export "check") (param $byte_len i32) (param $opcode i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $byte_len) (i32.const 2))
      (i32.or
        (i32.or
          (i32.or (i32.eq (local.get $opcode) (i32.const 1))
                  (i32.eq (local.get $opcode) (i32.const 2)))
          (i32.or (i32.eq (local.get $opcode) (i32.const 3))
                  (i32.eq (local.get $opcode) (i32.const 4))))
        (i32.or (i32.eq (local.get $opcode) (i32.const 5))
                (i32.eq (local.get $opcode) (i32.const 6))))))
)
"#;

pub const TFTP_ACK_LENGTH_WAT: &str = r#";; generated from tftp-rfc1350-ack-length-0001
;; expr: opcode != 4 || byte_len == 4
(module
  (func (export "check") (param $byte_len i32) (param $opcode i32) (result i32)
    (i32.or (i32.ne (local.get $opcode) (i32.const 4))
            (i32.eq (local.get $byte_len) (i32.const 4))))
)
"#;

pub const TFTP_DATA_LENGTH_WAT: &str = r#";; generated from tftp-rfc1350-data-length-0001
;; expr: opcode != 3 || byte_len >= 4
(module
  (func (export "check") (param $byte_len i32) (param $opcode i32) (result i32)
    (i32.or (i32.ne (local.get $opcode) (i32.const 3))
            (i32.ge_u (local.get $byte_len) (i32.const 4))))
)
"#;

pub const SHA1_FIPS180_FIXED_ABI_WAT: &str = r#";; sha1-fips180 fixed ABI unit
;; Input frame: u16 kind, u16 flags, u32 payload_len, payload bytes.
;; Output event: u16 kind=11, u16 flags=0, u32 len=20, 20 digest bytes.
(module
  (memory (export "memory") 1)

  (func $read16le (param $ptr i32) (result i32)
    (i32.or
      (i32.load8_u (local.get $ptr))
      (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8))))

  (func $read32le (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.load8_u (local.get $ptr))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))

  (func $read32be (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))

  (func $write16le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $value) (i32.const 8))))

  (func $write32le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.shr_u (local.get $value) (i32.const 24))))

  (func $write32be (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.shr_u (local.get $value) (i32.const 24)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (local.get $value)))

  (func $packed_result (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len))))

  (func $finish_error (param $code i32) (result i64)
    (call $write16le (i32.const 4096) (i32.const 12))
    (call $write16le (i32.const 4098) (i32.const 1))
    (call $write32le (i32.const 4100) (i32.const 4))
    (call $write32le (i32.const 4104) (local.get $code))
    (call $packed_result (i32.const 4096) (i32.const 12)))

  (func $finish_digest
    (param $h0 i32) (param $h1 i32) (param $h2 i32) (param $h3 i32) (param $h4 i32)
    (result i64)
    (call $write16le (i32.const 4096) (i32.const 11))
    (call $write16le (i32.const 4098) (i32.const 0))
    (call $write32le (i32.const 4100) (i32.const 20))
    (call $write32be (i32.const 4104) (local.get $h0))
    (call $write32be (i32.const 4108) (local.get $h1))
    (call $write32be (i32.const 4112) (local.get $h2))
    (call $write32be (i32.const 4116) (local.get $h3))
    (call $write32be (i32.const 4120) (local.get $h4))
    (call $packed_result (i32.const 4096) (i32.const 28)))

  (func (export "proto_abi_version") (result i32) (i32.const 1))
  (func (export "proto_standard_id") (result i32) (i32.const 1801))
  (func (export "proto_open") (param $config_ptr i32) (param $config_len i32) (result i32) (i32.const 1))
  (func (export "proto_close") (param $handle i32))
  (func (export "proto_pull") (param $handle i32) (result i64) (i64.const 0))

  (func (export "proto_push")
    (param $handle i32) (param $frame_ptr i32) (param $frame_len i32)
    (result i64)
    (local $kind i32) (local $payload_len i32) (local $payload_ptr i32)
    (local $padded_len i32) (local $src i32) (local $dst i32) (local $i i32)
    (local $chunk i32)
    (local $h0 i32) (local $h1 i32) (local $h2 i32) (local $h3 i32) (local $h4 i32)
    (local $a i32) (local $b i32) (local $c i32) (local $d i32) (local $e i32)
    (local $f i32) (local $k i32) (local $temp i32)

    (if (i32.ne (local.get $handle) (i32.const 1)) (then (return (call $finish_error (i32.const 3)))))
    (if (i32.lt_u (local.get $frame_len) (i32.const 8)) (then (return (call $finish_error (i32.const 1)))))
    (local.set $kind (call $read16le (local.get $frame_ptr)))
    (local.set $payload_len (call $read32le (i32.add (local.get $frame_ptr) (i32.const 4))))
    (if (i32.gt_u (local.get $payload_len) (i32.sub (local.get $frame_len) (i32.const 8)))
      (then (return (call $finish_error (i32.const 2)))))
    (if (i32.gt_u (local.get $payload_len) (i32.const 49000))
      (then (return (call $finish_error (i32.const 5)))))
    (if (i32.and (i32.ne (local.get $kind) (i32.const 1)) (i32.ne (local.get $kind) (i32.const 3)))
      (then (return (call $finish_error (i32.const 4)))))

    (local.set $payload_ptr (i32.add (local.get $frame_ptr) (i32.const 8)))
    (local.set $padded_len
      (i32.mul
        (i32.div_u (i32.add (local.get $payload_len) (i32.const 72)) (i32.const 64))
        (i32.const 64)))

    (local.set $src (local.get $payload_ptr))
    (local.set $dst (i32.const 8192))
    (local.set $i (i32.const 0))
    (loop $copy
      (if (i32.lt_u (local.get $i) (local.get $payload_len))
        (then
          (i32.store8 (i32.add (local.get $dst) (local.get $i))
            (i32.load8_u (i32.add (local.get $src) (local.get $i))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $copy))))
    (loop $zero
      (if (i32.lt_u (local.get $i) (local.get $padded_len))
        (then
          (i32.store8 (i32.add (local.get $dst) (local.get $i)) (i32.const 0))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $zero))))
    (i32.store8 (i32.add (local.get $dst) (local.get $payload_len)) (i32.const 128))
    (call $write32be (i32.add (local.get $dst) (i32.sub (local.get $padded_len) (i32.const 8))) (i32.const 0))
    (call $write32be (i32.add (local.get $dst) (i32.sub (local.get $padded_len) (i32.const 4)))
      (i32.shl (local.get $payload_len) (i32.const 3)))

    (local.set $h0 (i32.const 0x67452301))
    (local.set $h1 (i32.const 0xefcdab89))
    (local.set $h2 (i32.const 0x98badcfe))
    (local.set $h3 (i32.const 0x10325476))
    (local.set $h4 (i32.const 0xc3d2e1f0))

    (local.set $chunk (i32.const 0))
    (loop $chunks
      (if (i32.lt_u (local.get $chunk) (local.get $padded_len))
        (then
          (local.set $i (i32.const 0))
          (loop $wfirst
            (if (i32.lt_u (local.get $i) (i32.const 16))
              (then
                (i32.store
                  (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 2)))
                  (call $read32be
                    (i32.add (i32.add (i32.const 8192) (local.get $chunk))
                      (i32.shl (local.get $i) (i32.const 2)))))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $wfirst))))
          (loop $wrest
            (if (i32.lt_u (local.get $i) (i32.const 80))
              (then
                (i32.store
                  (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 2)))
                  (i32.rotl
                    (i32.xor
                      (i32.xor
                        (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 3)) (i32.const 2))))
                        (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 8)) (i32.const 2)))))
                      (i32.xor
                        (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 14)) (i32.const 2))))
                        (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 16)) (i32.const 2))))))
                    (i32.const 1)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $wrest))))

          (local.set $a (local.get $h0))
          (local.set $b (local.get $h1))
          (local.set $c (local.get $h2))
          (local.set $d (local.get $h3))
          (local.set $e (local.get $h4))

          (local.set $i (i32.const 0))
          (loop $rounds
            (if (i32.lt_u (local.get $i) (i32.const 80))
              (then
                (if (i32.lt_u (local.get $i) (i32.const 20))
                  (then
                    (local.set $f
                      (i32.or
                        (i32.and (local.get $b) (local.get $c))
                        (i32.and (i32.xor (local.get $b) (i32.const -1)) (local.get $d))))
                    (local.set $k (i32.const 0x5a827999))))
                (if
                  (i32.and
                    (i32.ge_u (local.get $i) (i32.const 20))
                    (i32.lt_u (local.get $i) (i32.const 40)))
                  (then
                    (local.set $f (i32.xor (i32.xor (local.get $b) (local.get $c)) (local.get $d)))
                    (local.set $k (i32.const 0x6ed9eba1))))
                (if
                  (i32.and
                    (i32.ge_u (local.get $i) (i32.const 40))
                    (i32.lt_u (local.get $i) (i32.const 60)))
                  (then
                    (local.set $f
                      (i32.or
                        (i32.or (i32.and (local.get $b) (local.get $c)) (i32.and (local.get $b) (local.get $d)))
                        (i32.and (local.get $c) (local.get $d))))
                    (local.set $k (i32.const 0x8f1bbcdc))))
                (if (i32.ge_u (local.get $i) (i32.const 60))
                  (then
                    (local.set $f (i32.xor (i32.xor (local.get $b) (local.get $c)) (local.get $d)))
                    (local.set $k (i32.const 0xca62c1d6))))
                (local.set $temp
                  (i32.add
                    (i32.add
                      (i32.add
                        (i32.add (i32.rotl (local.get $a) (i32.const 5)) (local.get $f))
                        (local.get $e))
                      (local.get $k))
                    (i32.load (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 2))))))
                (local.set $e (local.get $d))
                (local.set $d (local.get $c))
                (local.set $c (i32.rotl (local.get $b) (i32.const 30)))
                (local.set $b (local.get $a))
                (local.set $a (local.get $temp))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $rounds))))

          (local.set $h0 (i32.add (local.get $h0) (local.get $a)))
          (local.set $h1 (i32.add (local.get $h1) (local.get $b)))
          (local.set $h2 (i32.add (local.get $h2) (local.get $c)))
          (local.set $h3 (i32.add (local.get $h3) (local.get $d)))
          (local.set $h4 (i32.add (local.get $h4) (local.get $e)))
          (local.set $chunk (i32.add (local.get $chunk) (i32.const 64)))
          (br $chunks))))

    (call $finish_digest
      (local.get $h0) (local.get $h1) (local.get $h2) (local.get $h3) (local.get $h4)))
)
"#;

pub const SHA256_FIPS180_FIXED_ABI_WAT: &str = r#";; sha256-fips180 fixed ABI unit
;; Input frame: u16 kind, u16 flags, u32 payload_len, payload bytes.
;; Output event: u16 kind=11, u16 flags=0, u32 len=32, 32 digest bytes.
(module
  (memory (export "memory") 1)
  (data (i32.const 5120) "\98\2f\8a\42\91\44\37\71\cf\fb\c0\b5\a5\db\b5\e9\5b\c2\56\39\f1\11\f1\59\a4\82\3f\92\d5\5e\1c\ab\98\aa\07\d8\01\5b\83\12\be\85\31\24\c3\7d\0c\55\74\5d\be\72\fe\b1\de\80\a7\06\dc\9b\74\f1\9b\c1\c1\69\9b\e4\86\47\be\ef\c6\9d\c1\0f\cc\a1\0c\24\6f\2c\e9\2d\aa\84\74\4a\dc\a9\b0\5c\da\88\f9\76\52\51\3e\98\6d\c6\31\a8\c8\27\03\b0\c7\7f\59\bf\f3\0b\e0\c6\47\91\a7\d5\51\63\ca\06\67\29\29\14\85\0a\b7\27\38\21\1b\2e\fc\6d\2c\4d\13\0d\38\53\54\73\0a\65\bb\0a\6a\76\2e\c9\c2\81\85\2c\72\92\a1\e8\bf\a2\4b\66\1a\a8\70\8b\4b\c2\a3\51\6c\c7\19\e8\92\d1\24\06\99\d6\85\35\0e\f4\70\a0\6a\10\16\c1\a4\19\08\6c\37\1e\4c\77\48\27\b5\bc\b0\34\b3\0c\1c\39\4a\aa\d8\4e\4f\ca\9c\5b\f3\6f\2e\68\ee\82\8f\74\6f\63\a5\78\14\78\c8\84\08\02\c7\8c\fa\ff\be\90\eb\6c\50\a4\f7\a3\f9\be\f2\78\71\c6")

  (func $read16le (param $ptr i32) (result i32)
    (i32.or
      (i32.load8_u (local.get $ptr))
      (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8))))

  (func $read32le (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.load8_u (local.get $ptr))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))

  (func $read32be (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 24))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 16)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 8))
        (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))))))

  (func $write16le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $value) (i32.const 8))))

  (func $write32le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (i32.shr_u (local.get $value) (i32.const 24))))

  (func $write32be (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (i32.shr_u (local.get $value) (i32.const 24)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 1)) (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 2)) (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8 (i32.add (local.get $ptr) (i32.const 3)) (local.get $value)))

  (func $packed_result (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len))))

  (func $finish_error (param $code i32) (result i64)
    (call $write16le (i32.const 4096) (i32.const 12))
    (call $write16le (i32.const 4098) (i32.const 1))
    (call $write32le (i32.const 4100) (i32.const 4))
    (call $write32le (i32.const 4104) (local.get $code))
    (call $packed_result (i32.const 4096) (i32.const 12)))

  (func $finish_digest
    (param $h0 i32) (param $h1 i32) (param $h2 i32) (param $h3 i32)
    (param $h4 i32) (param $h5 i32) (param $h6 i32) (param $h7 i32)
    (result i64)
    (call $write16le (i32.const 4096) (i32.const 11))
    (call $write16le (i32.const 4098) (i32.const 0))
    (call $write32le (i32.const 4100) (i32.const 32))
    (call $write32be (i32.const 4104) (local.get $h0))
    (call $write32be (i32.const 4108) (local.get $h1))
    (call $write32be (i32.const 4112) (local.get $h2))
    (call $write32be (i32.const 4116) (local.get $h3))
    (call $write32be (i32.const 4120) (local.get $h4))
    (call $write32be (i32.const 4124) (local.get $h5))
    (call $write32be (i32.const 4128) (local.get $h6))
    (call $write32be (i32.const 4132) (local.get $h7))
    (call $packed_result (i32.const 4096) (i32.const 40)))

  (func (export "proto_abi_version") (result i32) (i32.const 1))
  (func (export "proto_standard_id") (result i32) (i32.const 180256))
  (func (export "proto_open") (param $config_ptr i32) (param $config_len i32) (result i32) (i32.const 1))
  (func (export "proto_close") (param $handle i32))
  (func (export "proto_pull") (param $handle i32) (result i64) (i64.const 0))

  (func (export "proto_push")
    (param $handle i32) (param $frame_ptr i32) (param $frame_len i32)
    (result i64)
    (local $kind i32) (local $payload_len i32) (local $payload_ptr i32)
    (local $padded_len i32) (local $src i32) (local $dst i32) (local $i i32)
    (local $chunk i32)
    (local $h0 i32) (local $h1 i32) (local $h2 i32) (local $h3 i32)
    (local $h4 i32) (local $h5 i32) (local $h6 i32) (local $h7 i32)
    (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    (local $e i32) (local $f i32) (local $g i32) (local $hh i32)
    (local $s0 i32) (local $s1 i32) (local $ch i32) (local $maj i32)
    (local $t1 i32) (local $t2 i32)

    (if (i32.ne (local.get $handle) (i32.const 1)) (then (return (call $finish_error (i32.const 3)))))
    (if (i32.lt_u (local.get $frame_len) (i32.const 8)) (then (return (call $finish_error (i32.const 1)))))
    (local.set $kind (call $read16le (local.get $frame_ptr)))
    (local.set $payload_len (call $read32le (i32.add (local.get $frame_ptr) (i32.const 4))))
    (if (i32.gt_u (local.get $payload_len) (i32.sub (local.get $frame_len) (i32.const 8)))
      (then (return (call $finish_error (i32.const 2)))))
    (if (i32.gt_u (local.get $payload_len) (i32.const 49000))
      (then (return (call $finish_error (i32.const 5)))))
    (if (i32.and (i32.ne (local.get $kind) (i32.const 1)) (i32.ne (local.get $kind) (i32.const 3)))
      (then (return (call $finish_error (i32.const 4)))))

    (local.set $payload_ptr (i32.add (local.get $frame_ptr) (i32.const 8)))
    (local.set $padded_len
      (i32.mul
        (i32.div_u (i32.add (local.get $payload_len) (i32.const 72)) (i32.const 64))
        (i32.const 64)))

    (local.set $src (local.get $payload_ptr))
    (local.set $dst (i32.const 8192))
    (local.set $i (i32.const 0))
    (loop $copy
      (if (i32.lt_u (local.get $i) (local.get $payload_len))
        (then
          (i32.store8 (i32.add (local.get $dst) (local.get $i))
            (i32.load8_u (i32.add (local.get $src) (local.get $i))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $copy))))
    (loop $zero
      (if (i32.lt_u (local.get $i) (local.get $padded_len))
        (then
          (i32.store8 (i32.add (local.get $dst) (local.get $i)) (i32.const 0))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $zero))))
    (i32.store8 (i32.add (local.get $dst) (local.get $payload_len)) (i32.const 128))
    (call $write32be (i32.add (local.get $dst) (i32.sub (local.get $padded_len) (i32.const 8))) (i32.const 0))
    (call $write32be (i32.add (local.get $dst) (i32.sub (local.get $padded_len) (i32.const 4)))
      (i32.shl (local.get $payload_len) (i32.const 3)))

    (local.set $h0 (i32.const 0x6a09e667)) (local.set $h1 (i32.const 0xbb67ae85))
    (local.set $h2 (i32.const 0x3c6ef372)) (local.set $h3 (i32.const 0xa54ff53a))
    (local.set $h4 (i32.const 0x510e527f)) (local.set $h5 (i32.const 0x9b05688c))
    (local.set $h6 (i32.const 0x1f83d9ab)) (local.set $h7 (i32.const 0x5be0cd19))

    (local.set $chunk (i32.const 0))
    (loop $chunks
      (if (i32.lt_u (local.get $chunk) (local.get $padded_len))
        (then
          (local.set $i (i32.const 0))
          (loop $wfirst
            (if (i32.lt_u (local.get $i) (i32.const 16))
              (then
                (i32.store
                  (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 2)))
                  (call $read32be
                    (i32.add (i32.add (i32.const 8192) (local.get $chunk))
                      (i32.shl (local.get $i) (i32.const 2)))))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $wfirst))))
          (loop $wrest
            (if (i32.lt_u (local.get $i) (i32.const 64))
              (then
                (local.set $s0
                  (i32.xor
                    (i32.xor
                      (i32.rotr (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 15)) (i32.const 2)))) (i32.const 7))
                      (i32.rotr (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 15)) (i32.const 2)))) (i32.const 18)))
                    (i32.shr_u (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 15)) (i32.const 2)))) (i32.const 3))))
                (local.set $s1
                  (i32.xor
                    (i32.xor
                      (i32.rotr (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 2)) (i32.const 2)))) (i32.const 17))
                      (i32.rotr (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 2)) (i32.const 2)))) (i32.const 19)))
                    (i32.shr_u (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 2)) (i32.const 2)))) (i32.const 10))))
                (i32.store
                  (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 2)))
                  (i32.add
                    (i32.add
                      (i32.add (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 16)) (i32.const 2)))) (local.get $s0))
                      (i32.load (i32.add (i32.const 4608) (i32.shl (i32.sub (local.get $i) (i32.const 7)) (i32.const 2)))))
                    (local.get $s1)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $wrest))))

          (local.set $a (local.get $h0)) (local.set $b (local.get $h1))
          (local.set $c (local.get $h2)) (local.set $d (local.get $h3))
          (local.set $e (local.get $h4)) (local.set $f (local.get $h5))
          (local.set $g (local.get $h6)) (local.set $hh (local.get $h7))
          (local.set $i (i32.const 0))
          (loop $rounds
            (if (i32.lt_u (local.get $i) (i32.const 64))
              (then
                (local.set $s1
                  (i32.xor
                    (i32.xor (i32.rotr (local.get $e) (i32.const 6)) (i32.rotr (local.get $e) (i32.const 11)))
                    (i32.rotr (local.get $e) (i32.const 25))))
                (local.set $ch (i32.xor (i32.and (local.get $e) (local.get $f)) (i32.and (i32.xor (local.get $e) (i32.const -1)) (local.get $g))))
                (local.set $t1
                  (i32.add
                    (i32.add
                      (i32.add
                        (i32.add
                          (i32.add (local.get $hh) (local.get $s1))
                          (local.get $ch))
                        (i32.load (i32.add (i32.const 5120) (i32.shl (local.get $i) (i32.const 2)))))
                      (i32.load (i32.add (i32.const 4608) (i32.shl (local.get $i) (i32.const 2)))))
                    (i32.const 0)))
                (local.set $s0
                  (i32.xor
                    (i32.xor (i32.rotr (local.get $a) (i32.const 2)) (i32.rotr (local.get $a) (i32.const 13)))
                    (i32.rotr (local.get $a) (i32.const 22))))
                (local.set $maj
                  (i32.xor
                    (i32.xor (i32.and (local.get $a) (local.get $b)) (i32.and (local.get $a) (local.get $c)))
                    (i32.and (local.get $b) (local.get $c))))
                (local.set $t2 (i32.add (local.get $s0) (local.get $maj)))
                (local.set $hh (local.get $g))
                (local.set $g (local.get $f))
                (local.set $f (local.get $e))
                (local.set $e (i32.add (local.get $d) (local.get $t1)))
                (local.set $d (local.get $c))
                (local.set $c (local.get $b))
                (local.set $b (local.get $a))
                (local.set $a (i32.add (local.get $t1) (local.get $t2)))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $rounds))))

          (local.set $h0 (i32.add (local.get $h0) (local.get $a)))
          (local.set $h1 (i32.add (local.get $h1) (local.get $b)))
          (local.set $h2 (i32.add (local.get $h2) (local.get $c)))
          (local.set $h3 (i32.add (local.get $h3) (local.get $d)))
          (local.set $h4 (i32.add (local.get $h4) (local.get $e)))
          (local.set $h5 (i32.add (local.get $h5) (local.get $f)))
          (local.set $h6 (i32.add (local.get $h6) (local.get $g)))
          (local.set $h7 (i32.add (local.get $h7) (local.get $hh)))
          (local.set $chunk (i32.add (local.get $chunk) (i32.const 64)))
          (br $chunks))))

    (call $finish_digest
      (local.get $h0) (local.get $h1) (local.get $h2) (local.get $h3)
      (local.get $h4) (local.get $h5) (local.get $h6) (local.get $h7)))
)
"#;

pub const UDP_RFC768_FIXED_ABI_WAT: &str = r#";; udp-rfc768 fixed ABI unit
;; Frame integers are little-endian. UDP wire fields remain RFC 768 big-endian.
;; Input frame:  u16 kind, u16 flags, u32 payload_len, UDP datagram bytes
;; Output event: u16 kind=11, u16 flags=status, u32 len=14,
;;               u16 status, source_port, destination_port, udp_length,
;;               payload_offset, payload_len, finding_bits
(module
  (memory (export "memory") 1)

  (func $read16le (param $ptr i32) (result i32)
    (i32.or
      (i32.load8_u (local.get $ptr))
      (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8))))

  (func $read32le (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.load8_u (local.get $ptr))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))

  (func $read16be (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  (func $write16le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $value) (i32.const 8))))

  (func $write32le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 2))
      (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 3))
      (i32.shr_u (local.get $value) (i32.const 24))))

  (func $packed_result (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len))))

  (func $finish_event
    (param $status i32) (param $src i32) (param $dst i32) (param $udp_len i32)
    (param $payload_offset i32) (param $payload_len i32) (param $bits i32)
    (result i64)
    (call $write16le (i32.const 4096) (i32.const 11))
    (call $write16le (i32.const 4098) (local.get $status))
    (call $write32le (i32.const 4100) (i32.const 14))
    (call $write16le (i32.const 4104) (local.get $status))
    (call $write16le (i32.const 4106) (local.get $src))
    (call $write16le (i32.const 4108) (local.get $dst))
    (call $write16le (i32.const 4110) (local.get $udp_len))
    (call $write16le (i32.const 4112) (local.get $payload_offset))
    (call $write16le (i32.const 4114) (local.get $payload_len))
    (call $write16le (i32.const 4116) (local.get $bits))
    (call $packed_result (i32.const 4096) (i32.const 22)))

  (func $finish_error (param $code i32) (result i64)
    (call $write16le (i32.const 4096) (i32.const 12))
    (call $write16le (i32.const 4098) (i32.const 1))
    (call $write32le (i32.const 4100) (i32.const 4))
    (call $write32le (i32.const 4104) (local.get $code))
    (call $packed_result (i32.const 4096) (i32.const 12)))

  (func (export "proto_abi_version") (result i32) (i32.const 1))
  (func (export "proto_standard_id") (result i32) (i32.const 768))
  (func (export "proto_open") (param $config_ptr i32) (param $config_len i32) (result i32)
    (i32.const 1))
  (func (export "proto_close") (param $handle i32))
  (func (export "proto_pull") (param $handle i32) (result i64) (i64.const 0))

  (func (export "proto_push")
    (param $handle i32) (param $frame_ptr i32) (param $frame_len i32)
    (result i64)
    (local $kind i32)
    (local $payload_len i32)
    (local $payload_ptr i32)
    (local $src i32)
    (local $dst i32)
    (local $udp_len i32)
    (local $length_ok i32)

    (if (i32.ne (local.get $handle) (i32.const 1))
      (then (return (call $finish_error (i32.const 3)))))
    (if (i32.lt_u (local.get $frame_len) (i32.const 8))
      (then (return (call $finish_error (i32.const 1)))))

    (local.set $kind (call $read16le (local.get $frame_ptr)))
    (local.set $payload_len (call $read32le (i32.add (local.get $frame_ptr) (i32.const 4))))
    (if (i32.gt_u (local.get $payload_len) (i32.sub (local.get $frame_len) (i32.const 8)))
      (then (return (call $finish_error (i32.const 2)))))
    (if
      (i32.and
        (i32.ne (local.get $kind) (i32.const 1))
        (i32.ne (local.get $kind) (i32.const 3)))
      (then (return (call $finish_error (i32.const 4)))))

    (local.set $payload_ptr (i32.add (local.get $frame_ptr) (i32.const 8)))
    (if (i32.lt_u (local.get $payload_len) (i32.const 8))
      (then
        (return
          (call $finish_event
            (i32.const 2) (i32.const 0) (i32.const 0) (i32.const 0)
            (i32.const 8) (i32.const 0) (i32.const 0)))))

    (local.set $src (call $read16be (local.get $payload_ptr)))
    (local.set $dst (call $read16be (i32.add (local.get $payload_ptr) (i32.const 2))))
    (local.set $udp_len (call $read16be (i32.add (local.get $payload_ptr) (i32.const 4))))
    (local.set $length_ok
      (i32.and
        (i32.ge_u (local.get $udp_len) (i32.const 8))
        (i32.eq (local.get $udp_len) (local.get $payload_len))))

    (call $finish_event
      (select (i32.const 0) (i32.const 2) (local.get $length_ok))
      (local.get $src)
      (local.get $dst)
      (local.get $udp_len)
      (i32.const 8)
      (i32.sub (local.get $payload_len) (i32.const 8))
      (local.get $length_ok)))
)
"#;

pub const TFTP_RFC1350_FIXED_ABI_WAT: &str = r#";; tftp-rfc1350 fixed ABI unit
;; Frame integers are little-endian. TFTP wire fields remain RFC 1350 big-endian.
;; Input frame:  u16 kind, u16 flags, u32 payload_len, payload bytes
;; Output event: u16 kind=11, u16 flags=status, u32 len=8,
;;               u16 status, u16 opcode, u16 input_len, u16 finding_bits
(module
  (memory (export "memory") 1)

  (func $read16le (param $ptr i32) (result i32)
    (i32.or
      (i32.load8_u (local.get $ptr))
      (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8))))

  (func $read32le (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.load8_u (local.get $ptr))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.or
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 16))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 24)))))

  (func $read16be (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  (func $write16le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $value) (i32.const 8))))

  (func $write32le (param $ptr i32) (param $value i32)
    (i32.store8 (local.get $ptr) (local.get $value))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 1))
      (i32.shr_u (local.get $value) (i32.const 8)))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 2))
      (i32.shr_u (local.get $value) (i32.const 16)))
    (i32.store8
      (i32.add (local.get $ptr) (i32.const 3))
      (i32.shr_u (local.get $value) (i32.const 24))))

  (func $packed_result (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len))))

  (func $finish_event
    (param $status i32) (param $opcode i32) (param $input_len i32) (param $bits i32)
    (result i64)
    (call $write16le (i32.const 4096) (i32.const 11))
    (call $write16le (i32.const 4098) (local.get $status))
    (call $write32le (i32.const 4100) (i32.const 8))
    (call $write16le (i32.const 4104) (local.get $status))
    (call $write16le (i32.const 4106) (local.get $opcode))
    (call $write16le (i32.const 4108) (local.get $input_len))
    (call $write16le (i32.const 4110) (local.get $bits))
    (call $packed_result (i32.const 4096) (i32.const 16)))

  (func $finish_error (param $code i32) (result i64)
    (call $write16le (i32.const 4096) (i32.const 12))
    (call $write16le (i32.const 4098) (i32.const 1))
    (call $write32le (i32.const 4100) (i32.const 4))
    (call $write32le (i32.const 4104) (local.get $code))
    (call $packed_result (i32.const 4096) (i32.const 12)))

  (func (export "proto_abi_version") (result i32)
    (i32.const 1))

  (func (export "proto_standard_id") (result i32)
    (i32.const 1350))

  (func (export "proto_open") (param $config_ptr i32) (param $config_len i32) (result i32)
    (i32.const 1))

  (func (export "proto_close") (param $handle i32))

  (func (export "proto_pull") (param $handle i32) (result i64)
    (i64.const 0))

  (func (export "proto_push")
    (param $handle i32) (param $frame_ptr i32) (param $frame_len i32)
    (result i64)
    (local $kind i32)
    (local $payload_len i32)
    (local $payload_ptr i32)
    (local $opcode i32)
    (local $opcode_ok i32)
    (local $ack_ok i32)
    (local $data_ok i32)
    (local $bits i32)
    (local $all_ok i32)

    (if (i32.ne (local.get $handle) (i32.const 1))
      (then (return (call $finish_error (i32.const 3)))))
    (if (i32.lt_u (local.get $frame_len) (i32.const 8))
      (then (return (call $finish_error (i32.const 1)))))

    (local.set $kind (call $read16le (local.get $frame_ptr)))
    (local.set $payload_len (call $read32le (i32.add (local.get $frame_ptr) (i32.const 4))))
    (if (i32.gt_u (local.get $payload_len) (i32.sub (local.get $frame_len) (i32.const 8)))
      (then (return (call $finish_error (i32.const 2)))))
    (if
      (i32.and
        (i32.ne (local.get $kind) (i32.const 1))
        (i32.ne (local.get $kind) (i32.const 3)))
      (then (return (call $finish_error (i32.const 4)))))

    (local.set $payload_ptr (i32.add (local.get $frame_ptr) (i32.const 8)))
    (if (i32.lt_u (local.get $payload_len) (i32.const 2))
      (then
        (return
          (call $finish_event
            (i32.const 2)
            (i32.const 0)
            (local.get $payload_len)
            (i32.const 0)))))

    (local.set $opcode (call $read16be (local.get $payload_ptr)))
    (local.set $opcode_ok
      (i32.and
        (i32.ge_u (local.get $opcode) (i32.const 1))
        (i32.le_u (local.get $opcode) (i32.const 6))))
    (local.set $ack_ok
      (i32.or
        (i32.ne (local.get $opcode) (i32.const 4))
        (i32.eq (local.get $payload_len) (i32.const 4))))
    (local.set $data_ok
      (i32.or
        (i32.ne (local.get $opcode) (i32.const 3))
        (i32.ge_u (local.get $payload_len) (i32.const 4))))
    (local.set $bits
      (i32.or
        (i32.or
          (local.get $opcode_ok)
          (i32.shl (local.get $ack_ok) (i32.const 1)))
        (i32.shl (local.get $data_ok) (i32.const 2))))
    (local.set $all_ok
      (i32.and
        (i32.and (local.get $opcode_ok) (local.get $ack_ok))
        (local.get $data_ok)))

    (call $finish_event
      (select (i32.const 0) (i32.const 2) (local.get $all_ok))
      (local.get $opcode)
      (local.get $payload_len)
      (local.get $bits)))
)
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ack_passes() {
        let bytes = [0, 4, 0, 1];
        let report = analyze_tftp(&bytes);
        assert_eq!(report.exit_code(), 0);
        assert_eq!(report.finding_count(), 3);
    }

    #[test]
    fn invalid_ack_length_rejects() {
        let bytes = [0, 4, 0, 1, 0];
        let report = analyze_tftp(&bytes);
        assert_eq!(report.exit_code(), 2);
        assert_eq!(report.finding_count(), 3);
    }

    #[test]
    fn sha256_empty_matches_standard_vector() {
        assert_eq!(
            sha256(b""),
            [
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
                0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
                0x78, 0x52, 0xb8, 0x55
            ]
        );
    }
}
