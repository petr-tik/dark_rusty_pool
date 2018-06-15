cargo test -q
cargo build -q --release
free && sync && sudo sh -c "echo 3 > /proc/sys/vm/drop_caches" && free
time ./target/release/order_book 200 < data/pricer.in &> /dev/null
