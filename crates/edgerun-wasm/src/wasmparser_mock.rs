// Mock wasmparser module — minimal stubs so the codebase compiles without the real crate.
// These types mirror the public API surface used in this workspace.
// They do NOT implement real WASM parsing; call sites that rely on them
// will need to be stubbed or feature-gated separately.

pub mod wasmparser {
    use std::marker::PhantomData;

    #[derive(Debug, Clone, Copy)]
    pub enum ValType {
        I32,
        I64,
        F32,
        F64,
        V128,
        Ref(RefType),
    }

    #[derive(Debug, Clone, Copy)]
    pub struct RefType;

    #[derive(Debug, Clone)]
    pub struct FuncType {
        params: Vec<ValType>,
        results: Vec<ValType>,
    }

    impl FuncType {
        pub fn params(&self) -> &[ValType] {
            &self.params
        }
        pub fn results(&self) -> &[ValType] {
            &self.results
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ExternalKind {
        Func,
        Table,
        Memory,
        Global,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum TypeRef {
        Func(u32),
        Table,
        Memory,
        Global(u32),
    }

    #[derive(Debug, Clone)]
    pub enum Payload<'a> {
        TypeSection(TypeSection<'a>),
        ImportSection(ImportSection<'a>),
        FunctionSection(FunctionSection<'a>),
        ExportSection(ExportSection<'a>),
        CodeSectionEntry(CodeSectionEntry<'a>),
        GlobalSection(GlobalSection<'a>),
        End,
        _Phantom(PhantomData<&'a ()>),
    }

    #[derive(Debug, Clone)]
    pub struct TypeSection<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> TypeSection<'a> {
        pub fn into_iter(self) -> TypeSectionIntoIter<'a> {
            TypeSectionIntoIter { _marker: PhantomData }
        }
    }

    impl<'a> IntoIterator for TypeSection<'a> {
        type Item = Result<TypeEntry<'a>, anyhow::Error>;
        type IntoIter = TypeSectionIntoIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.into_iter()
        }
    }

    pub struct TypeSectionIntoIter<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for TypeSectionIntoIter<'a> {
        type Item = Result<TypeEntry<'a>, anyhow::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    #[derive(Debug, Clone)]
    pub struct TypeEntry<'a> {
        pub composite_type: CompositeType,
        _marker: PhantomData<&'a ()>,
    }

    #[derive(Debug, Clone)]
    pub struct CompositeType {
        pub func: Option<FuncType>,
    }

    impl CompositeType {
        pub fn unwrap_func(&self) -> &FuncType {
            self.func.as_ref().unwrap()
        }
    }

    #[derive(Debug, Clone)]
    pub struct ImportSection<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> ImportSection<'a> {
        pub fn into_iter(self) -> ImportSectionIntoIter<'a> {
            ImportSectionIntoIter { _marker: PhantomData }
        }
    }

    impl<'a> IntoIterator for ImportSection<'a> {
        type Item = Result<Import<'a>, anyhow::Error>;
        type IntoIter = ImportSectionIntoIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.into_iter()
        }
    }

    pub struct ImportSectionIntoIter<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for ImportSectionIntoIter<'a> {
        type Item = Result<Import<'a>, anyhow::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    #[derive(Debug, Clone)]
    pub struct Import<'a> {
        pub module: &'a str,
        pub name: &'a str,
        pub ty: TypeRef,
    }

    #[derive(Debug, Clone)]
    pub struct FunctionSection<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> FunctionSection<'a> {
        pub fn into_iter(self) -> FunctionSectionIntoIter {
            FunctionSectionIntoIter { _marker: PhantomData }
        }
    }

    impl<'a> IntoIterator for FunctionSection<'a> {
        type Item = Result<u32, anyhow::Error>;
        type IntoIter = FunctionSectionIntoIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.into_iter()
        }
    }

    pub struct FunctionSectionIntoIter<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for FunctionSectionIntoIter<'a> {
        type Item = Result<u32, anyhow::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    #[derive(Debug, Clone)]
    pub struct ExportSection<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> ExportSection<'a> {
        pub fn into_iter(self) -> ExportSectionIntoIter<'a> {
            ExportSectionIntoIter { _marker: PhantomData }
        }
    }

    impl<'a> IntoIterator for ExportSection<'a> {
        type Item = Result<Export<'a>, anyhow::Error>;
        type IntoIter = ExportSectionIntoIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.into_iter()
        }
    }

    pub struct ExportSectionIntoIter<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for ExportSectionIntoIter<'a> {
        type Item = Result<Export<'a>, anyhow::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    #[derive(Debug, Clone)]
    pub struct Export<'a> {
        pub name: &'a str,
        pub kind: ExternalKind,
        pub index: u32,
    }

    #[derive(Debug, Clone)]
    pub struct CodeSectionEntry<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> CodeSectionEntry<'a> {
        pub fn get_locals_reader(&self) -> LocalsReader {
            LocalsReader
        }
        pub fn get_operators_reader(&self) -> OperatorsReader {
            OperatorsReader
        }
    }

    pub struct LocalsReader;

    impl LocalsReader {
        pub fn get_count(&self) -> u32 {
            0
        }
        pub fn read(&mut self) -> Result<(u32, ValType), anyhow::Error> {
            Err(anyhow::anyhow!("mock"))
        }
    }

    pub struct OperatorsReader;

    impl OperatorsReader {
        pub fn eof(&self) -> bool {
            true
        }
        pub fn read(&mut self) -> Result<Operator, anyhow::Error> {
            Err(anyhow::anyhow!("mock"))
        }
    }

    #[derive(Debug, Clone)]
    pub struct GlobalSection<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> GlobalSection<'a> {
        pub fn into_iter(self) -> GlobalSectionIntoIter<'a> {
            GlobalSectionIntoIter { _marker: PhantomData }
        }
    }

    impl<'a> IntoIterator for GlobalSection<'a> {
        type Item = Result<Global<'a>, anyhow::Error>;
        type IntoIter = GlobalSectionIntoIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.into_iter()
        }
    }

    pub struct GlobalSectionIntoIter<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for GlobalSectionIntoIter<'a> {
        type Item = Result<Global<'a>, anyhow::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    #[derive(Debug, Clone)]
    pub struct Global<'a> {
        pub ty: GlobalType,
        pub init_expr: ConstExpr<'a>,
    }

    #[derive(Debug, Clone)]
    pub struct GlobalType {
        pub content_type: ValType,
    }

    // AOT-related types

    #[derive(Debug, Clone, Copy)]
    pub enum BlockType {
        Empty,
        Result(ValType),
        Type(u32),
    }

    #[derive(Debug, Clone, Copy)]
    pub struct MemArg {
        pub offset: u64,
        pub align: u32,
        pub memory_index: u32,
    }

    #[derive(Debug, Clone)]
    pub enum Operator {
        Unreachable,
        Nop,
        Block(BlockType),
        Loop(BlockType),
        If(BlockType),
        Else,
        End,
        Br(u32),
        BrIf(u32),
        BrTable(BrTable),
        Return,
        Call(u32),
        CallIndirect(CallIndirect),
        Drop,
        Select,
        LocalGet(u32),
        LocalSet(u32),
        LocalTee(u32),
        GlobalGet(u32),
        GlobalSet(u32),
        I32Load(MemArg),
        I64Load(MemArg),
        F32Load(MemArg),
        F64Load(MemArg),
        I32Load8U(MemArg),
        I32Load8S(MemArg),
        I32Load16U(MemArg),
        I32Load16S(MemArg),
        I64Load8U(MemArg),
        I64Load16U(MemArg),
        I64Load32U(MemArg),
        I32Store(MemArg),
        I64Store(MemArg),
        F32Store(MemArg),
        F64Store(MemArg),
        I32Store8(MemArg),
        I32Store16(MemArg),
        MemoryCopy { dst_mem: u32, src_mem: u32 },
        MemoryFill { mem: u32 },
        I32Const(i32),
        I64Const(i64),
        F32Const(f32),
        F64Const(f64),
        I32Eqz,
        I32Eq,
        I32Ne,
        I32LtS,
        I32LtU,
        I32GtS,
        I32GtU,
        I32LeS,
        I32LeU,
        I32GeS,
        I32GeU,
        I64Eqz,
        I64Eq,
        I64Ne,
        I64LtS,
        I64LtU,
        I64GtS,
        I64GtU,
        I64LeS,
        I64LeU,
        I64GeS,
        I64GeU,
        F32Eq,
        F32Ne,
        F32Lt,
        F32Gt,
        F32Le,
        F32Ge,
        F64Eq,
        F64Ne,
        F64Lt,
        F64Gt,
        F64Le,
        F64Ge,
        I32Add,
        I32Sub,
        I32Mul,
        I32DivS,
        I32DivU,
        I32RemS,
        I32RemU,
        I32And,
        I32Or,
        I32Xor,
        I32Shl,
        I32ShrS,
        I32ShrU,
        I32Rotl,
        I32Rotr,
        I64Add,
        I64Sub,
        I64Mul,
        I64DivS,
        I64DivU,
        I64RemS,
        I64RemU,
        I64And,
        I64Or,
        I64Xor,
        I64Shl,
        I64ShrS,
        I64ShrU,
        I64Rotl,
        I64Rotr,
        F32Add,
        F32Sub,
        F32Mul,
        F32Div,
        F32Min,
        F32Max,
        F32Copysign,
        F64Add,
        F64Sub,
        F64Mul,
        F64Div,
        F64Min,
        F64Max,
        F64Copysign,
        I32WrapI64,
        I64ExtendI32S,
        I64ExtendI32U,
        F32DemoteF64,
        F64PromoteF32,
        F32ConvertI32S,
        F32ConvertI32U,
        F32ConvertI64S,
        F32ConvertI64U,
        F64ConvertI32S,
        F64ConvertI32U,
        F64ConvertI64S,
        F64ConvertI64U,
        I32TruncF32S,
        I32TruncF32U,
        I32TruncF64S,
        I32TruncF64U,
        I64TruncF32S,
        I64TruncF32U,
        I64TruncF64S,
        I64TruncF64U,
        MemorySize(u32),
        MemoryGrow(u32),
        _Other,
    }

    #[derive(Debug, Clone)]
    pub struct BrTable {
        pub targets: Vec<u32>,
        pub default: u32,
    }

    #[derive(Debug, Clone)]
    pub struct CallIndirect {
        pub type_index: u32,
        pub table_index: u32,
    }

    #[derive(Debug, Clone)]
    pub struct ConstExpr<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> ConstExpr<'a> {
        pub fn get_operators_reader(&self) -> ConstExprOperatorsReader {
            ConstExprOperatorsReader
        }
    }

    pub struct ConstExprOperatorsReader;

    impl ConstExprOperatorsReader {
        pub fn read(&mut self) -> Result<Operator, anyhow::Error> {
            Err(anyhow::anyhow!("mock"))
        }
        pub fn eof(&self) -> bool {
            true
        }
    }

    pub struct Parser;

    impl Parser {
        pub fn new(_offset: usize) -> Self {
            Self
        }
        pub fn parse_all<'a>(&self, _bytes: &'a [u8]) -> ParseAll<'a> {
            ParseAll { _marker: PhantomData }
        }
    }

    pub struct ParseAll<'a> {
        _marker: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for ParseAll<'a> {
        type Item = Result<Payload<'a>, anyhow::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    pub struct Validator;

    impl Validator {
        pub fn new() -> Self {
            Self
        }
        pub fn validate_all(&self, _bytes: &[u8]) -> Result<(), anyhow::Error> {
            Ok(())
        }
    }
}
