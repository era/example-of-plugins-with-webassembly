cargo build --release --target wasm32-wasi
wasm-tools component new ./target/wasm32-wasi/debug/hello_world_plugin.wasm \
    -o hello-world.wasm --adapt wasi_snapshot_preview1=./wasi_snapshot_preview1.reactor.wasm

mv hello-world.wasm ../host-application/plugins