# rainbeam-plugins

`rainbeam-plugins` is used to load plugins in the form of WASM files. Each plugin is loaded through a TOML file placed in `./.config/plugins`.

Optionally, servers can choose to verify loaded plugins through [Neospring](https://neospring.org) assets. The Neospring API will be called to check the marketplace ID that is linked through the `asset` field of the plugin config TOML file. If the content of this asset does not match a checksum of the WASM file, then the plugin will fail to load.
