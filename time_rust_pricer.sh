cargo test
cargo build --release
time ./target/release/order_book 200 < data/pricer.in &> /dev/null
