# Pong Royale

## Running
### run desktop client using udp:
```
cargo run -- --client 1
```
### run server with udp:
```
cargo run -- --server
```
### run wasm client using web-rtc:
```
cargo build --release --target wasm32-unknown-unknown --no-default-features --features web; wasm-bindgen --no-typescript --target web --out-name wasm --out-dir target/distribution target/wasm32-unknown-unknown/release/pong-royale.wasm; copy .\target\distribution\wasm* . ; simple-http-server.exe
```
### run server with web-rtc
```
cargo run --no-default-features --features headless -- --server
```
