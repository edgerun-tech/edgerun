// Mock wasmtime module — minimal stubs so the codebase compiles without the real crate.
// These types mirror the public API surface used in this workspace.
// They do NOT implement real WASM execution; call sites that rely on them
// will need to be stubbed or feature-gated separately.

use std::sync::{Arc, Mutex};

pub struct Engine;

impl Engine {
    pub fn default() -> Self {
        Self
    }
}

pub struct Module;

impl Module {
    pub fn from_file(_engine: &Engine, _path: &std::path::Path) -> Result<Self, anyhow::Error> {
        Ok(Self)
    }
}

pub struct Store<T> {
    _data: T,
}

impl<T> Store<T> {
    pub fn new(_engine: &Engine, data: T) -> Self {
        Self { _data: data }
    }
}

pub struct Linker<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> Linker<T> {
    pub fn new(_engine: &Engine) -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn func_wrap<F>(
        &mut self,
        _module: &str,
        _name: &str,
        _func: F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut() + 'static,
    {
        Ok(())
    }

    pub fn instantiate(
        &self,
        _store: &mut Store<T>,
        _module: &Module,
    ) -> Result<Instance, anyhow::Error> {
        Ok(Instance)
    }
}

pub struct Instance;

impl Instance {
    pub fn get_typed_func<Params, Results>(
        &self,
        _store: &mut Store<impl std::any::Any>,
        _name: &str,
    ) -> Result<TypedFunc<Params, Results>, anyhow::Error> {
        Err(anyhow::anyhow!("mock: no functions available"))
    }

    pub fn get_memory(
        &self,
        _store: &mut Store<impl std::any::Any>,
        _name: &str,
    ) -> Option<Memory> {
        None
    }
}

pub struct TypedFunc<Params, Results> {
    _params: std::marker::PhantomData<Params>,
    _results: std::marker::PhantomData<Results>,
}

impl<Params, Results> TypedFunc<Params, Results> {
    pub fn call(
        &self,
        _store: &mut Store<impl std::any::Any>,
        _params: Params,
    ) -> Result<Results, anyhow::Error>
    where
        Params: std::any::Any,
        Results: Default,
    {
        Ok(Results::default())
    }
}

pub struct Memory;

impl Memory {
    pub fn data_size(&self, _store: &mut Store<impl std::any::Any>) -> u64 {
        0
    }

    pub fn grow(
        &self,
        _store: &mut Store<impl std::any::Any>,
        _pages: u64,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn data_mut<'a>(&self, _store: &'a mut Store<impl std::any::Any>) -> &'a mut [u8] {
        &mut []
    }
}

pub struct Caller<'a, T> {
    _store: &'a mut Store<T>,
}

impl<'a, T> Caller<'a, T> {
    pub fn get_export(&self, _name: &str) -> Option<Extern> {
        None
    }
}

#[derive(Debug)]
pub struct Extern;

impl Extern {
    pub fn into_memory(self) -> Option<Memory> {
        None
    }
}

// Re-export for convenience
// (Removed incorrect pub use - types are accessed via wasmtime_mock::*)
