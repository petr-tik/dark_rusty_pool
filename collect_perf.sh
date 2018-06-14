sudo perf record -o perf.data -F 99 -g ./target/release/order_book 200 < data/pricer.in > /dev/null
sudo perf script -i perf.data | stackcollapse-perf.pl | ./demangle_rust.sh > out.folded
grep "order_book" out.folded | flamegraph.pl > order-book-pricer-rust.svg
