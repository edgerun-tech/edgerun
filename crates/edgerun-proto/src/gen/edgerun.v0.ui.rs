// @generated
// UI protocol types — proto-v0:ui
// Deterministic UI description produced by WASM, rendered by platform adapters.

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UINode {
    #[prost(string, tag = "1")]
    pub node_type: ::prost::alloc::string::String,

    #[prost(btree_map = "string, string", tag = "2")]
    pub props: ::prost::alloc::collections::BTreeMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,

    #[prost(message, repeated, tag = "3")]
    pub children: ::prost::alloc::vec::Vec<UINode>,

    #[prost(string, optional, tag = "4")]
    pub action: ::core::option::Option<::prost::alloc::string::String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UIActionEvent {
    #[prost(uint32, tag = "1")]
    pub event_version: u32,

    #[prost(string, tag = "2")]
    pub action: ::prost::alloc::string::String,

    #[prost(string, optional, tag = "3")]
    pub node_path: ::core::option::Option<::prost::alloc::string::String>,

    #[prost(string, optional, tag = "4")]
    pub context: ::core::option::Option<::prost::alloc::string::String>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UIRenderRequest {
    #[prost(uint32, tag = "1")]
    pub request_version: u32,

    #[prost(string, optional, tag = "2")]
    pub last_action: ::core::option::Option<::prost::alloc::string::String>,

    #[prost(uint64, optional, tag = "3")]
    pub state_version: ::core::option::Option<u64>,
}

// @@protoc_insertion_point(module)
