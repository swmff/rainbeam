pub mod config;

use wasmtime::{
    Engine, Store,
    component::{Linker, Component, Val},
};
use std::io::{Error, ErrorKind, Result};

pub fn run(cnf: config::PluginConfig) -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let bytes = std::fs::read(&cnf.wasm)?;
    let component = Component::new(&engine, bytes).expect("failed to load wasm component");

    // linker
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("name", move |_store, _params: ()| {
            Ok((String::from(cnf.name.clone()),))
        })
        .unwrap();

    // create store and pull "plugin" function
    let instance = linker
        .instantiate(&mut store, &component)
        .expect("failed to create instance");

    let plugin_entry = instance
        .get_func(&mut store, "plugin")
        .expect("plugin entry not found");

    let mut result = [Val::List(Vec::new())];
    if let Err(e) = plugin_entry.call(&mut store, &[], &mut result) {
        return Err(Error::new(ErrorKind::Other, e.to_string()));
    };

    Ok(())
}
