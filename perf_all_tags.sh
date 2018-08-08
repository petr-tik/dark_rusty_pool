for crt_tag in $(git tag)
do
  git checkout ${crt_tag}
  cargo clean
  cargo build --release 2>/dev/null
  sync && sudo sh -c "echo 3 > /proc/sys/vm/drop_caches"
  sudo perf stat -e cpu-clock,task-clock,cs,cache-references,cache-misses,branches,branch-misses,instructions,cycles ./target/release/order_book 200 < data/pricer.in > /dev/null
done
git checkout master
