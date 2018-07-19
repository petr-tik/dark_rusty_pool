gunzip data/pricer.in.gz
cargo test
cargo build -q --release
cargo run --release 200 < data/pricer.in > /tmp/rust_pricer_out_200
zcat data/pricer.out.200.gz | diff -s /tmp/rust_pricer_out -
cargo run --release 10000 < data/pricer.in > /tmp/rust_pricer_out_10000
zcat data/pricer.out.10000.gz | diff -s /tmp/rust_pricer_out_10000 -

