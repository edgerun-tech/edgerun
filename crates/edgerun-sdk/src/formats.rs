pub fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn parse_api(bytes: &[u8]) -> Option<edgerun_wire::UnitApi> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<edgerun_wire::SdkWireRecord, edgerun_wire::WireError>(&owned)
        .ok()?
    {
        edgerun_wire::SdkWireRecord::UnitApi(api)
            if api.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION =>
        {
            Some(api)
        }
        _ => None,
    }
}

pub fn parse_composition(bytes: &[u8]) -> Option<edgerun_wire::CompositionRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<edgerun_wire::SdkWireRecord, edgerun_wire::WireError>(&owned)
        .ok()?
    {
        edgerun_wire::SdkWireRecord::Composition(composition)
            if composition.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && composition.flags & 1 == 1 =>
        {
            Some(composition)
        }
        _ => None,
    }
}

pub fn parse_report(bytes: &[u8]) -> Option<edgerun_wire::ExecutionReportRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<edgerun_wire::SdkWireRecord, edgerun_wire::WireError>(&owned)
        .ok()?
    {
        edgerun_wire::SdkWireRecord::ExecutionReport(report)
            if report.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && report.flags & 1 == 1 =>
        {
            Some(report)
        }
        _ => None,
    }
}

pub fn parse_segment_report(bytes: &[u8]) -> Option<edgerun_wire::SegmentReportRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<edgerun_wire::SdkWireRecord, edgerun_wire::WireError>(&owned)
        .ok()?
    {
        edgerun_wire::SdkWireRecord::SegmentReport(report)
            if report.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && report.flags & 1 == 1 =>
        {
            Some(report)
        }
        _ => None,
    }
}

pub fn parse_segment(bytes: &[u8]) -> Option<edgerun_wire::SegmentRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<edgerun_wire::SdkWireRecord, edgerun_wire::WireError>(&owned)
        .ok()?
    {
        edgerun_wire::SdkWireRecord::Segment(segment)
            if segment.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && segment.flags & 1 == 1 =>
        {
            Some(segment)
        }
        _ => None,
    }
}

pub fn parse_chain(bytes: &[u8]) -> Option<edgerun_wire::ChainRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<edgerun_wire::SdkWireRecord, edgerun_wire::WireError>(&owned)
        .ok()?
    {
        edgerun_wire::SdkWireRecord::Chain(chain)
            if chain.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION && chain.flags & 1 == 1 =>
        {
            Some(chain)
        }
        _ => None,
    }
}
