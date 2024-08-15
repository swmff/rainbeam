# build release
build database="sqlite":
    cargo build -r --no-default-features --features {{database}}

# build debug
build-d:
    cargo build

# test
test:
    cargo run

# ...
doc:
    cargo doc --no-deps --document-private-items

subs:
    git submodule update --recursive --remote

pull:
    git pull
    git submodule update