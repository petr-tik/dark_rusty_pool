## tl;dr

Learnt Rust to implement an order book and compare against another implementation in C++17. 

## Lightning talk

Gave a 5-minute talk about this project at the London Rust meetup. 

[Slides](https://docs.google.com/presentation/d/e/2PACX-1vShyqTQMgiZyg7GpxN5cqOqKM-cLAVhvymcDQFCp4gRcLubBz7OuoL3houVt_HdDmsCbOxMbF3KbWyl/pub?start=false&loop=false&delayms=3000)

## Requirements

  * Rust nightly
  * fnv 1.0.6
  * Cargo

## Test and run

Git clone and change into the directory, before running the command below. The first argument is the target size of the order book. After that the executable will wait for market data feed on stdin. Conveniently packaged in a shell script.

```bash
cargo test
cargo build --release
cargo run --release <target_size> < data/<market_data_file>
```

Test harness from the problem statement. Writes output to tmp files and compares to expected output files. 

```bash
./run_basic_test.sh
./run_big_test.sh
```

## Design

The order book allows adding new orders, reducing current ones and printing the amount earned from selling <target\_size> of shares or amount spent on buying <target\_size> of shares.

```rust
type Depth = i64;

struct OrderBook<T: IdPriceCache + Sized> {
    cache: T,
    bids_at_price: BTreeMap<Amount, Depth>,
    bids_total_size: i64,
    asks_at_price: BTreeMap<Amount, Depth>,
    asks_total_size: i64,
    target_size: i64,
    // only 1 side is affected on Reduce or Limit order
    last_action_side: OrderSide,   // which side was touched last
    last_action_timestamp: String, // timestamp of last touched side
}
```

## API

### Adding a new limit order

Parse the price point (input as float) and quantity (relevant optimisation below). Find the relevant order side and insert the key-value pair of (price, depth). Update the last action side and last action timestamp as well. 


### Reducing an order

Using the order id, look up in cache, the side and price of the order. Find the relevant bucket inside the ordered map of the given side, decrement the depth of the bucket.

### Checking and reporting

To check if the order book needs to report income/expense, you need to see if the last affected side now has total depth more than target_size. Only if it does, do we calculate the amount. 

Keeping total depth per price, gives us a shortcut to quickly calculate how much we can make/spend on each bucket as `size * price`.

### Storage

Orders are stored in:

  * Cache to look up price and side by order id
  * Ordered map (BTreeMap) of prices to depths.

## Implemented perf improvements

Benchmarking my first implementation against Ludwig's C++17 version showed that my design was terrible. Performance optimisations that I made (chronological order):

  0. Realised this from the start - store float prices as ints. FP arithmetic is more CPU-intensive. For printing - implemeted own Display trait turns int into a string and prints the string with a separator <decimal_points> chars away from the right-hand side. Input: 44.25, stored as 4425, printed as "44.25".

### Collection method

Made defined a bash for-loop to iterate over git tags and  measure application performance using `perf stat`. 

```bash
for crt_tag in $(git tag)
do
  git checkout ${crt_tag}
  cargo clean
  # checkout tag, rebuild the project completely
  cargo build --release 2>/dev/null
  # flush virtual memory and drop caches
  # otherwise later runs will have faster memory access to data
  sync && sudo sh -c "echo 3 > /proc/sys/vm/drop_caches"
  # perf deserves another explanation
  # measure context switches, time, cache hits/misses, IPC
  sudo perf stat -e cpu-clock,task-clock,cs,cache-references,cache-misses,branches,branch-misses,instructions,cycles ./target/release/order_book 200 < data/pricer.in > /dev/null
done
git checkout master
```


  0. Passed tests, but took forever. `v0.1`
  
```bash
 Performance counter stats for './target/release/order_book 200':

      50537.486947      cpu-clock (msec)          #    1.000 CPUs utilized          
      50537.414869      task-clock (msec)         #    1.000 CPUs utilized          
               571      cs                        #    0.011 K/sec                  
     9,998,564,647      cache-references          #  197.845 M/sec                    (83.34%)
     2,215,945,360      cache-misses              #   22.163 % of all cache refs      (83.33%)
    19,648,873,036      branches                  #  388.798 M/sec                    (83.33%)
        17,591,410      branch-misses             #    0.09% of all branches          (66.67%)
    61,199,665,090      instructions              #    0.62  insn per cycle           (83.33%)
    98,967,973,922      cycles                    #    1.958 GHz                      (83.33%)

      50.550863035 seconds time elapsed
```

  1. First implementation stored full limit order structs in Linked Lists in BTreeMaps. Linked list nodes were heap-allocated and blew the cache efficiency of my algrorithm. Ultimatelly, it's not necessary to keep the exact order. I now use the BTreeMap as a key value store between price point and depth of order book at that price point. `v0.2`

```bash
 Performance counter stats for './target/release/order_book 200':

       2076.217308      cpu-clock (msec)          #    0.998 CPUs utilized          
       2076.216247      task-clock (msec)         #    0.998 CPUs utilized          
                13      cs                        #    0.006 K/sec                  
        25,284,833      cache-references          #   12.178 M/sec                    (83.30%)
        11,798,266      cache-misses              #   46.661 % of all cache refs      (83.24%)
     1,618,768,908      branches                  #  779.672 M/sec                    (83.24%)
        11,094,665      branch-misses             #    0.69% of all branches          (66.83%)
     8,635,599,563      instructions              #    2.10  insn per cycle           (83.43%)
     4,121,813,900      cycles                    #    1.985 GHz                      (83.39%)

       2.080010478 seconds time elapsed
```

  2. After running `collect_perf` and `perf report`, I found that println! was taking 8.96% of time. Googling for efficient stdout printing in Rust suggested replacing println! with writeln! with a stdout lock as one of the args. `v0.3`

```bash
 Performance counter stats for './target/release/order_book 200':

       2085.846921      cpu-clock (msec)          #    0.998 CPUs utilized          
       2085.840465      task-clock (msec)         #    0.998 CPUs utilized          
                28      cs                        #    0.013 K/sec                  
        26,481,770      cache-references          #   12.696 M/sec                    (83.39%)
        12,163,530      cache-misses              #   45.932 % of all cache refs      (83.32%)
     1,615,246,371      branches                  #  774.385 M/sec                    (83.32%)
        10,825,763      branch-misses             #    0.67% of all branches          (66.64%)
     8,618,251,904      instructions              #    2.08  insn per cycle           (83.32%)
     4,141,705,180      cycles                    #    1.986 GHz                      (83.34%)

       2.089816968 seconds time elapsed
```

  3. Replaced Strings for timestamps with int64. Strings are heap-allocated, require malloc and free. Ints should be faster to allocate. Updated the benchmark. `v0.4`
  
```bash
 Performance counter stats for './target/release/order_book 200':

       1919.612102      cpu-clock (msec)          #    0.997 CPUs utilized          
       1919.578195      task-clock (msec)         #    0.997 CPUs utilized          
               276      cs                        #    0.144 K/sec                  
        24,954,872      cache-references          #   13.000 M/sec                    (83.36%)
        12,037,684      cache-misses              #   48.238 % of all cache refs      (83.15%)
     1,531,488,144      branches                  #  797.818 M/sec                    (83.36%)
        10,203,489      branch-misses             #    0.67% of all branches          (66.74%)
     7,935,234,326      instructions              #    2.08  insn per cycle           (83.38%)
     3,816,053,806      cycles                    #    1.988 GHz                      (83.39%)

       1.926124513 seconds time elapsed
```

  4. Since order IDs aren't required for stdout, we don't need to keep the string representation of each order id. I implemented a hash function (using a FNVHasher) and changed order id from `String` to `u64` in ReduceOrder and LimitOrder. Also changed the IdPriceCache signature to make sure cache looks `hash(id)` rather than `id: String`. `v0.5`

```bash 
 Performance counter stats for './target/release/order_book 200':

       1719.801389      cpu-clock (msec)          #    0.998 CPUs utilized          
       1719.798529      task-clock (msec)         #    0.998 CPUs utilized          
                15      cs                        #    0.009 K/sec                  
        19,151,428      cache-references          #   11.136 M/sec                    (83.25%)
         9,599,433      cache-misses              #   50.124 % of all cache refs      (83.26%)
     1,418,678,693      branches                  #  824.909 M/sec                    (83.30%)
        10,157,808      branch-misses             #    0.72% of all branches          (66.98%)
     7,257,090,913      instructions              #    2.12  insn per cycle           (83.49%)
     3,419,942,443      cycles                    #    1.989 GHz                      (83.21%)

       1.723019265 seconds time elapsed
```

  5. Enabled LTO and added compile-time detail to heap-allocated data structures that expand at runtime. Calling `malloc` often will increase time spent in kernel-space. Given that we know the size of input data, we can call malloc at application start-up, request a lot of memory at once to reduce future calls for additional memory. The trade-off between requesting too much memory at start-up that you will never need vs. calling malloc for every expansion of BTreeMap can be investigated with different input sizes. `v0.6`
  
```bash 
 Performance counter stats for './target/release/order_book 200':

       1639.816836      cpu-clock (msec)          #    0.998 CPUs utilized          
       1639.815687      task-clock (msec)         #    0.998 CPUs utilized          
                30      cs                        #    0.018 K/sec                  
        19,713,362      cache-references          #   12.022 M/sec                    (83.18%)
         9,871,063      cache-misses              #   50.073 % of all cache refs      (83.41%)
     1,260,727,374      branches                  #  768.822 M/sec                    (83.41%)
        10,015,820      branch-misses             #    0.79% of all branches          (66.84%)
     6,433,079,853      instructions              #    1.97  insn per cycle           (83.42%)
     3,259,360,232      cycles                    #    1.988 GHz                      (83.16%)

       1.642901598 seconds time elapsed
```

## Perf improvements to investigate

1. Currently - reducing an order into oblivion (eg. reduce an order of size 100, by >100) doesn't remove its key from the IdPriceCache. This leads to higher memory usage, if unused keys persist in the cache. It might be useful to remove the key-value pair, if the order is ever completely reduced. 

Requires: 

  * Adding order size to the IdOrderPrice cache and decrementing it after every reduce. 
  * Turning OrderBook.reduce() into reducing 2 internal states - not a pretty abstraction.

Pros:

  * if a lookup of previously-deleted key occurs, we can end that branch of logic quickly. Unlikely to occur - clients shouldn't ask to reduce the same order twice.
  * Prevents the BTreeMap from growing too much. Shouldn't matter too much, but on big applications, it's worth preserving heap space for ids with valid data.

2. Check if using a vector for bids and asks is better than a BTreeMap. Perf shows BTreeMap iterators to be one of the most expensive parts of the code and if the vector is cheaper to rewrite in practice - use the vector for cache locality.

## Motivation

Inspired by [Ludwig Pacifici's implementation using C++17](https://github.com/ludwigpacifici/order-book-pricer).
