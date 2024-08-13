# build release
build:
    cargo build -r

# build debug
build-d:
    cargo build

# test
test:
    cargo run

# ...
doc:
    cargo doc --no-deps --document-private-items
