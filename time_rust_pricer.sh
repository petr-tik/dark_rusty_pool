cargo test -q
cargo build -q --release
free && sync && sudo sh -c "echo 3 > /proc/sys/vm/drop_caches" && free
echo "Benchmarking with target_size: 200"
time ./target/release/order_book 200 < data/pricer.in &> /dev/null
free && sync && sudo sh -c "echo 3 > /proc/sys/vm/drop_caches"
echo "Benchmarking with target_size: 10000"
time ./target/release/order_book 10000 < data/pricer.in &> /dev/null
