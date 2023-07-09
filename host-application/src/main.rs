mod plugin;

#[tokio::main]
async fn main() {
    let result = plugin::WasmModule::new("plugins/hello-world.wasm")
        .unwrap()
        .invoke("world!")
        .await
        .unwrap();
    println!("{result}");
}
