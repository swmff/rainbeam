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

# test
test:
    just build-assets
    cargo run --bin rainbeam

# ...
doc:
    cargo doc --no-deps --document-private-items --workspace --exclude neospring-desktop

publish-shared:
    cargo publish --package rainbeam-shared

publish-databeam:
    cargo publish --package databeam

publish-langbeam:
    cargo publish --package langbeam

publish-citrus:
    cargo publish --package citrus

publish:
    just publish-shared
    just publish-databeam
    just publish-langbeam
    just publish-citrus
