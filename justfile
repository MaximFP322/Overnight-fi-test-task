export RUSTFLAGS := "-C target-cpu=native"
export CARGO_PROFILE_RELEASE_LTO := "fat"
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS := "1"
export LOCAL_UID := `id -u`
export LOCAL_GID := `id -g`

alias build := build-native

run:
    cargo run --release --bin overnight-fi-test-task

run-docker:
    mkdir -p logs
    docker compose up --build --exit-code-from overnight-fi-test-task

build-native:
    cargo build --release --bin overnight-fi-test-task

build-docker:
    docker compose build

bench-orderbook *args:
    cargo bench --bench orderbook -- {{args}}

bench-orderbook-allocs *args:
    cargo bench --bench orderbook --features bench-allocations -- --test {{args}}

stop-docker:
    docker compose down
