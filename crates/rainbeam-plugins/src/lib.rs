pub mod config;

use wasmtime::{Linker, Engine, Module, Store};
use std::io::{Error, ErrorKind, Result};

pub fn run(cnf: config::PluginConfig) -> Result<()> {
    let engine = Engine::default();
    let module = match Module::from_file(&engine, cnf.wasm) {
        Ok(m) => m,
        Err(e) => panic!("{e}"),
    };

    let linker = Linker::new(&engine);

    // create store and pull "plugin" function
    let mut store: Store<u32> = Store::new(&engine, 4);
    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(e) => panic!("{e}"),
    };

    let plugin_entry = match instance.get_typed_func::<(), ()>(&mut store, "plugin") {
        Ok(f) => f,
        Err(e) => panic!("{e}"),
    };

    if let Err(e) = plugin_entry.call(&mut store, ()) {
        return Err(Error::new(ErrorKind::Other, e.to_string()));
    };

    Ok(())
}
