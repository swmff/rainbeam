# set shell := ["pwsh.exe", "-c"] # For development on Windows, using PowerShell

# build release
build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}} --bin rainbeam

mimalloc-build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}},mimalloc --bin rainbeam

init-web:
    cd crates/web && npm i && cd ../../

web-dev:
    just web-bindings
    cd crates/web && bun run dev

web-build:
    just web-bindings
    cd crates/web && bun run build

web-bindings:
    cargo test

# build debug
build-d:
    just build-assets
    cargo build --bin rainbeam

# test
test-api:
    cargo run --bin rainbeam

# prod
api:
    ./target/release/rainbeam

web:
    cd crates/web && bun run ./build/index.js

# ...
doc:
    cargo doc --no-deps --document-private-items --workspace --exclude neospring-desktop

clean-deps:
    cargo upgrade -i
    cargo machete

publish-shared:
    cargo publish --package rainbeam-shared

publish-databeam:
    cargo publish --package databeam

publish-authbeam:
    cargo publish --package authbeam --no-verify

publish-langbeam:
    cargo publish --package langbeam

publish-citrus:
    cargo publish --package citrus-client --no-verify

publish:
    just publish-shared
    just publish-databeam
    just publish-authbeam
    just publish-langbeam
    just publish-citrus
