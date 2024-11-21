# steps to run the rust+bevy+wasm web projects

0.1 `cargo install -f wasm-bindgen-cli` - install latest wasm bindgen cli

-- if you get stuck here, I had to install a specific version e.g. `cargo install -f wasm-bindgen-cli --version 0.2.93` but this is already outdated, better try to keep things up to date.

0.2 `rustup target add wasm32-unknown-unknown` - done once in the project directory to add web support

1.1 `cargo build --release --target wasm32-unknown-unknown` - done every time you want to compile to build for the web

1.2 `wasm-bindgen --out-dir ./out/ --target web ./target/wasm32-unknown-unknown/release/your-game-name.wasm` - done every time to generate wasm bindings for the web (make sure your-game-name.wasm is set to your project name specified in Cargo.toml)

1.3 Now you can copy `./out` to `./web/out` and then

1.4 run the project locally with `npx serve web`

Open localhost on the specified port and see the Rust+Bevy wasm web project locally