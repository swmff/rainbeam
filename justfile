# build release
build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}} --bin rainbeam

mimalloc-build database="sqlite":
    just build-assets
    cargo build -r --no-default-features --features {{database}},mimalloc --bin rainbeam

init-builder:
    cd crates/builder && npm i && cd ../../

build-assets:
    node ./crates/builder/index.js

# build debug
build-d:
    just build-assets
    cargo build --bin rainbeam

# web
web-dev:
    cd crates/rainbeam-web && bun --bun run dev

web-build:
    cd crates/rainbeam-web && bun --bun run build

web:
    cd crates/rainbeam-web && bun ./build/index.js

# test
test:
    just build-assets
    cargo run --bin rainbeam

# ...
doc:
    cargo doc --no-deps --document-private-items --workspace --exclude neospring-desktop
