# set shell := ["pwsh.exe", "-c"] # For development on Windows, using PowerShell

# build release
build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}},redis --bin rainbeam

mimalloc-build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}},mimalloc,redis --bin rainbeam

moka-build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}},mimalloc,moka --bin rainbeam

init-builder:
    cd crates/builder && npm i && cd ../../

build-assets:
    node ./crates/builder/index.js

# build debug
build-d:
    just build-assets
    cargo build --bin rainbeam

# test
test:
    just build-assets
    cargo run --bin rainbeam

# test (profiling)
test-perf:
    just build-assets
    cargo build --bin rainbeam
    perf record ./target/debug/rainbeam

# ...
doc:
    cargo doc --no-deps --document-private-items --workspace --exclude neospring-desktop

clean-deps:
    cargo upgrade -i
    cargo machete

fix:
    cargo fix --allow-dirty
    cargo clippy --fix --allow-dirty

help-rust-analyzer-wont-work:
    # ...yeah I bet it doesn't with 236,366 files (175 GB) sitting about in target
    cargo clean

publish-shared:
    cargo publish --package rainbeam-shared

publish-databeam:
    cargo publish --package databeam

publish-authbeam:
    cargo publish --package authbeam --no-verify

publish-langbeam:
    cargo publish --package langbeam

publish:
    just publish-shared
    just publish-databeam
    just publish-authbeam
    just publish-langbeam
