# build release
build database="sqlite":
    cargo build -r --no-default-features --features {{database}} --bin rainbeam

# build debug
build-d:
    cargo build --bin rainbeam

# test
test:
    cargo run --bin rainbeam

# ...
doc:
    cargo doc --no-deps --document-private-items --workspace --exclude neospring-desktop
