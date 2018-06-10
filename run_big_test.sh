gunzip data/pricer.in.gz
cargo test
cargo build --release
echo "Running release binary with 200 as target size and saving output to /tmp/rust_pricer"
cargo run --release 200 < data/pricer.in > /tmp/rust_pricer
echo "to view results, cat/head /tmp/rust_pricer"
