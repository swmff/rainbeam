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
