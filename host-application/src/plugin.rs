use thiserror::Error;
use wasmtime::component::Component;
use wasmtime::component::Linker;
use wasmtime::component::Val;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2 as wasi_preview2;
use wasmtime_wasi::preview2::WasiCtx;
use wasmtime_wasi::preview2::WasiCtxBuilder;

wasmtime::component::bindgen!({world: "plugin"});

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Generic error `{0}`")]
    GenericError(String),
}

pub struct WasmModule {
    module: Component,
    linker: Linker<PluginRuntime>,
    engine: Engine,
}

struct PluginRuntime {
    wasi_ctx: WasiCtx,
    table: wasi_preview2::Table,
}

impl PluginRuntime {
    pub fn new() -> Self {
        let mut table = wasmtime_wasi::preview2::Table::new();
        Self {
            wasi_ctx: WasiCtxBuilder::new().build(&mut table).unwrap(),
            table,
        }
    }
}

// Implements the Trait which represents the functions
// the module imported
impl crate::plugin::host::Host for PluginRuntime {
    fn log(&mut self, txt: String) -> wasmtime::Result<()> {
        println!("{txt}");
        Ok(())
    }
}

// We need to impelement this "view" for
// wasi_preview2::wasi::command::add_to_linker
// it will expose the wasi ctx to our Wasm module.
impl wasi_preview2::WasiView for PluginRuntime {
    fn table(&self) -> &wasi_preview2::Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut wasi_preview2::Table {
        &mut self.table
    }

    fn ctx(&self) -> &wasi_preview2::WasiCtx {
        &self.wasi_ctx
    }

    fn ctx_mut(&mut self) -> &mut wasi_preview2::WasiCtx {
        &mut self.wasi_ctx
    }
}

impl WasmModule {
    pub fn new(path: &str) -> Result<Self, WasmError> {
        // An engine stores and configures global compilation settings like
        // optimization level, enabled wasm features, etc.
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config)
            .map_err(|e| WasmError::GenericError(format!("{} {}", e.to_string(), path)))?;

        // We start off by creating a `Module` which represents a compiled form
        // of our input wasm module. In this case it'll be JIT-compiled after
        // we parse the text format.
        //could use from_binary as well
        let module = Component::from_file(&engine, path)
            .map_err(|e| WasmError::GenericError(format!("{} {}", e.to_string(), path)))?;

        let mut linker = Linker::new(&engine);

        // Links our Runtime exposing the methods to our module
        Plugin::add_to_linker(&mut linker, |state: &mut PluginRuntime| state)
            .map_err(|e| WasmError::GenericError(e.to_string()))?;

        // since we are using wasi, we need to add the functions related with it
        // wasi_preview2 is a helper that will do that for us
        wasi_preview2::wasi::command::add_to_linker(&mut linker)
            .map_err(|e| WasmError::GenericError(e.to_string()))?;

        Ok(Self {
            module,
            linker,
            engine,
        })
    }

    // invoke our plugin (the `run` function)
    pub async fn invoke(&mut self, arg: &str) -> Result<String, WasmError> {
        // A `Store` is what will own instances, functions, globals, etc. All wasm
        // items are stored within a `Store`, and it's what we'll always be using to
        // interact with the wasm world. Custom data can be stored in stores but for
        // now we just use `()`.
        let mut store = Store::new(&self.engine, PluginRuntime::new());

        // With a compiled `Module` we can then instantiate it, creating
        // an `Instance` which we can actually poke at functions on.
        let instance = self
            .linker
            .instantiate_async(&mut store, &self.module)
            .await
            .map_err(|e| WasmError::GenericError(e.to_string()))?;

        // our result will be here
        let mut result = [Val::S32(0)];
        // let's get the function we are interested and call it
        instance
            .get_func(&mut store, "run")
            .unwrap()
            .call_async(&mut store, &mut [Val::String(arg.into())], &mut result)
            .await
            .map_err(|e| WasmError::GenericError(e.to_string()))?;

        Ok(format!("{:?}", result))
    }
}
