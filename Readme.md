## tl;dr

Learnt Rust to implement an order book and compare against another implementation in C++17. My rust application is slower than C++17 - benchmarks to come. Suspect the design is more of a culprit. Order book spec attached to avoid repeating myself.

## Requirements

  * Rust 1.26 - stdlib only
  * Cargo

## Test and run

Git clone and change into the directory, before running the command below. The first argument is the target size of the order book. After that the executable will wait for market data feed on stdin. Conveniently packaged in a shell script.

```bash
./run_basic_test.sh
```

## Integration test and benchmark

I compared the output with [Ludwig Pacifici's implementation in C++17](https://github.com/ludwigpacifici/order-book-pricer). We got the same results on pricer.in.gz

```bash
./run_big_test.sh
```

I compiled and ran Ludwig's version to another temp file and compared the output. We got the same result.

## Design objective

Keep the complexity of adding and reducing orders within a predictable range - minimise jitter. Orders are stored in LinkedLists inside ordered maps. Each linked list is indexable by the price of the order. 

## Design

The order book consists of 5 attributes and 4 data structures. 

```rust
struct OrderBook {
    cache: IdPriceCache,
    bids_at_price: BTreeMap<Amount, OrdersAtPrice>,
    bids_total_size: i64,
    asks_at_price: BTreeMap<Amount, OrdersAtPrice>,
    asks_total_size: i64,
    target_size: i64,
    // only 1 side is affected on Reduce or Limit order
    last_action_side: OrderSide,   // which side was touched last
    last_action_timestamp: String, // timestamp of last touched side
}
```

the BTreeMap takes most of the load on adding. 

### Adding a new limit order

Appending to the LinkedList is O(1), so the most expensive operation is indexing into the right linked list - find by price.

Each linked list also keeps track of total depth of orders at a given price. This becomes useful for checking and reporting.

### Reducing an order

Using the order id, look up in cache, the side and price of the order. Find the relevant LinkedList inside the ordered map of the given side, reduce the order by walking over the linked list until the given node is found. 


### Checking and reporting

To check if the order book needs to report income/expense, you need to see if the last affected side now has total depth more than target_size. Only if it does, do we calculate the amount. 

Keeping total depth per price, gives us a shortcut to quickly calculate how much we can make/spend on each bucket as `size * price`.

### Storage

Orders are stored in:

  * cache to look up price and side by order id
  * Ordered map (BTreeMap) of prices to OrdersAtPrice (doubly linked list)

## Motivation

Inspired by [Ludwig Pacifici's implementation using C++17](https://github.com/ludwigpacifici/order-book-pricer), I decided to learn Rust and implement an order book.
