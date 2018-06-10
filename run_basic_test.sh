cargo test
cargo build --release
cargo run --release 200 < data/basic.in.txt > /tmp/rust_basic
diff -s /tmp/rust_basic data/basic.out.txt
