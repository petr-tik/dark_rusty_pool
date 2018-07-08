gunzip data/pricer.in.gz
cargo test
cargo build --release
echo "Running release binary with 200 as target size and saving output to /tmp/rust_pricer_out_200"
cargo run --release 200 < data/pricer.in > /tmp/rust_pricer_out_200
zcat data/pricer.out.200.gz | diff -s /tmp/rust_pricer_out -
echo "to view results, cat/head /tmp/rust_pricer"
echo "Running release binary with 10000 as target size and saving output to /tmp/rust_pricer_out_10000"
cargo run --release 10000 < data/pricer.in > /tmp/rust_pricer_out_10000
zcat data/pricer.out.10000.gz | diff -s /tmp/rust_pricer_out_10000 -

