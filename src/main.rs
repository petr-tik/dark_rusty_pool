extern crate fnv;

use std::cmp::{min, max, Ordering, PartialEq, PartialOrd};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::prelude::*;

mod amount;
use amount::Amount;

mod orderside;
use orderside::OrderSide;

pub fn hash<T: Hash>(x: T) -> u64 {
    let mut hasher = fnv::FnvHasher::default();
    x.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
struct ReduceOrder {
    // "28800744 R b 20"
    timestamp: i64,
    id: u64,
    size: i64,
}

impl ReduceOrder {
    fn new(input_vec: Vec<&str>) -> Self {
        ReduceOrder {
            timestamp: input_vec[0].parse::<i64>().unwrap_or(0),
            id: hash(&input_vec[2]),
            size: input_vec[3].parse::<i64>().unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LimitOrder {
    // "28800538 A b S 44.26 100"
    timestamp: i64,
    id: u64,
    side: OrderSide,
    price: Amount,
    size: i64,
}

impl LimitOrder {
    fn new(input_vec: Vec<&str>) -> Self {
        LimitOrder {
            timestamp: input_vec[0].parse::<i64>().unwrap_or(0),
            id: hash(input_vec[2]),
            side: match input_vec[3] {
                "B" => OrderSide::Bid,
                "S" => OrderSide::Ask,
                _ => panic!("Couldn't parse order side from {}", input_vec[3]),
            },
            price: Amount::new_from_str(&input_vec[4]),
            size: input_vec[5].parse::<i64>().unwrap_or(0),
        }
    }
}

/// Price cache strategy (for benchmarking)
trait IdPriceCache {
    fn insert(&mut self, order: &LimitOrder);
    fn contains_key(&self, key: &u64) -> bool;
    fn get(&self, key: &u64) -> Option<&(Amount, OrderSide)>;
}

type IdPriceCacheFnvMap = fnv::FnvHashMap<u64, (Amount, OrderSide)>;
impl IdPriceCache for IdPriceCacheFnvMap {
    fn insert(&mut self, order: &LimitOrder) {
        self.insert(order.id, (order.price, order.side));
    }
    fn contains_key(&self, key: &u64) -> bool {
        self.contains_key(key)
    }
    fn get(&self, key: &u64) -> Option<&(Amount, OrderSide)> {
        self.get(key)
    }
}

type Depth = i64;

type BidsVec = Vec<Amount>;
type AsksVec = Vec<Amount>;
type DepthsVec = Vec<Depth>;


struct OrderBook<T: IdPriceCache + Sized> {
    cache: T,
    bids_at_price: BTreeMap<Amount, Depth>,
    bids_total_size: i64,
    bids_max: usize,
    asks_at_price: BTreeMap<Amount, Depth>,
    asks_total_size: i64,
    asks_max: usize,
    target_size: i64,
    // only 1 side is affected on Reduce or Limit order
    last_action_side: OrderSide, // which side was touched last
    last_action_timestamp: i64,  // timestamp of last touched side
}

impl<T: IdPriceCache + Sized> OrderBook<T> {
    fn new(target_size: i64, cache: T) -> Self {
        OrderBook {
            cache,
            bids_at_price: BTreeMap::new(),
            asks_at_price: BTreeMap::new(),
            bids_total_size: 0,
            asks_total_size: 0,
            target_size,
            last_action_side: OrderSide::Ask,
            last_action_timestamp: 000_000_000,
            bids_max: 0,
            asks_max: 0,
        }
    }

    fn add(&mut self, order: LimitOrder) {
        if order.side == OrderSide::Bid {
            *self.bids_at_price.entry(order.price).or_insert(0) += &order.size;
            self.bids_total_size += order.size;
            self.bids_max = max(self.bids_max, self.bids_at_price.len());
        } else if order.side == OrderSide::Ask {
            *self.asks_at_price.entry(order.price).or_insert(0) += &order.size;
            self.asks_total_size += order.size;
            self.asks_max = max(self.asks_max, self.asks_at_price.len());
        }
        self.cache.insert(&order);
        self.last_action_timestamp = order.timestamp;
        self.last_action_side = order.side;
    }

    fn reduce_order(&mut self, order: &ReduceOrder) {
        let (price, side) = match self.cache.get(&order.id) {
            Some(tup) => (tup),
            None => panic!("No order under key {}", &order.id),
        };
        if side == &OrderSide::Ask {
            if let Some(depth) = self.asks_at_price.get_mut(&price) {
                *depth -= &order.size;
                self.asks_total_size -= order.size;
            }
        } else if side == &OrderSide::Bid {
            if let Some(depth) = self.bids_at_price.get_mut(&price) {
                *depth -= &order.size;
                self.bids_total_size -= order.size;
            }
        }
        self.last_action_timestamp = order.timestamp;
        self.last_action_side = *side;
    }

    fn summarise_target(&self) -> Option<Amount> {
        /*
        Summarises income gained from selling self.target_size of shares or expense of buying self.target_size shares. If last side is Bid/Buy - we need to summarise 

        Relies on change of state of last_action_side attribute. 
        Every add_order and reduce order api call need to update the last_action_side.

        Returning None, means there aren't enough bids to sell to 
        or asks to buy.

         */
        if self.bids_total_size >= self.target_size && self.last_action_side == OrderSide::Bid {
            return Some(self.summarise_amount_from_bids());
        } else if self.asks_total_size >= self.target_size
            && self.last_action_side == OrderSide::Ask
        {
            return Some(self.summarise_amount_from_asks());
        }
        None
    }

    fn summarise_amount_from_asks(&self) -> Amount {
        let mut res = Amount::new();
        let mut target_left = self.target_size;
        for (price, depth) in &self.asks_at_price {
            if target_left <= 0 {
                break;
            }
            if *depth == 0 {
                continue;
            }
            let available_in_this_bucket = min(*depth, target_left);

            res += *price * available_in_this_bucket;
            target_left -= available_in_this_bucket;
        }

        res
    }

    fn summarise_amount_from_bids(&self) -> Amount {
        let mut res = Amount::new();
        let mut target_left = self.target_size;
        // BTreeMap is sorted in ascending order
        // but you want to sell in descending order
        // (higher is better)
        for (price, depth) in self.bids_at_price.iter().rev() {
            if target_left <= 0 {
                break;
            }
            if *depth == 0 {
                continue;
            }
            let available_in_this_bucket = min(*depth, target_left);
            res += *price * available_in_this_bucket;
            target_left -= available_in_this_bucket;
        }
        res
    }

    fn process(&mut self, instruction: &str) {
        let order_vec: Vec<&str> = instruction.trim().split(' ').collect();
        if order_vec[1] == "A" {
            self.add(LimitOrder::new(order_vec));
        } else if order_vec[1] == "R" {
            self.reduce_order(&ReduceOrder::new(order_vec));
        } else {
            eprintln!("Error processing {}", instruction);
        }
    }
}

/// Returns the target size for the order book.
/// Takes env args and parses them into a i64
/// Panics when no target size is provided or parsing fails
fn get_target_size() -> i64 {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Need target size input");
    }
    match args[1].parse::<i64>() {
        Ok(res) => res,
        Err(_e) => panic!("Couldn't parse input into i64"),
    }
}

fn prepare_reports() -> HashMap<OrderSide, Option<Amount>> {
    let mut hm = HashMap::with_capacity(2);
    hm.insert(OrderSide::Ask, None);
    hm.insert(OrderSide::Bid, None);

    hm
}

fn main() {
    let target_size = get_target_size();
    let cache_capacity = 50000;
    let mut ob = OrderBook::new(
        target_size,
        IdPriceCacheFnvMap::with_capacity_and_hasher(
            cache_capacity,
            std::hash::BuildHasherDefault::<fnv::FnvHasher>::default(),
        ),
    );
    let mut reports = prepare_reports();
    let stdout = io::stdout();
    let stdin = io::stdin();
    for order_line in stdin.lock().lines() {
        let unwrapped_line: &str = &order_line.unwrap();
        ob.process(unwrapped_line);

        let cur = ob.summarise_target();
        let mut prev = reports.get_mut(&!ob.last_action_side).unwrap();
        if cur.is_some() && prev.is_some() {
            if cur.unwrap() != prev.unwrap() {
                writeln!(
                    stdout.lock(),
                    "{} {} {}",
                    ob.last_action_timestamp,
                    !ob.last_action_side,
                    cur.unwrap()
                ).expect("cannot lock");
            } else {
                continue;
            }
        } else if cur.is_some() && prev.is_none() {
            writeln!(
                stdout.lock(),
                "{} {} {}",
                ob.last_action_timestamp,
                !ob.last_action_side,
                cur.unwrap()
            ).expect("cannot lock");
        } else if cur.is_none() && prev.is_some() {
            writeln!(
                stdout.lock(),
                "{} {} NA",
                ob.last_action_timestamp,
                !ob.last_action_side
            ).expect("cannot lock");
        }
        *prev = cur;
    }
    println!("max size of bids: {}, max size of asks: {}", ob.bids_max, ob.asks_max);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_order_constructor() {
        let lo = LimitOrder::new("28800538 A b S 44.07 100".split(' ').collect());
        assert_eq!(lo.timestamp, 28800538);
        assert_eq!(lo.id, hash("b"));
        assert_eq!(lo.side, OrderSide::Ask);
        assert_eq!(lo.price.as_int, 4407);
        assert_eq!(lo.size, 100);
    }

    #[test]
    fn orderbook_constructor_works() {
        let target_size = 500;
        let ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.target_size, target_size);
        assert_eq!(ob.last_action_timestamp, 000000000);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
    }

    #[test]
    fn orderbook_add_ask() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        let ask = LimitOrder::new("28800538 A b S 44.26 100".split(' ').collect());
        ob.add(ask);
        assert_eq!(ob.asks_total_size, 100);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.summarise_target(), None);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, 28800538);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert!(ob.asks_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_add_bid() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        let bid = LimitOrder::new("28800538 A b B 44.26 100".split(' ').collect());
        ob.add(bid);
        assert_eq!(ob.bids_total_size, 100);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.summarise_target(), None);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800538);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert!(ob.bids_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_reduce_ask() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        let ask = LimitOrder::new("28800538 A b S 44.26 100".split(' ').collect());
        ob.add(ask);
        let ro = ReduceOrder::new("28800744 R b 20".split(' ').collect());
        ob.reduce_order(&ro);
        assert_eq!(ob.asks_total_size, 80);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.summarise_target(), None);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, 28800744);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert!(ob.asks_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_reduce_bid() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        let bid = LimitOrder::new("28800538 A b B 44.26 100".split(' ').collect());
        ob.add(bid);
        let ro = ReduceOrder::new("28800744 R b 20".split(' ').collect());
        ob.reduce_order(&ro);
        assert_eq!(ob.bids_total_size, 80);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800744);
        assert_eq!(ob.summarise_target(), None);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert!(ob.bids_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_add_reduce_add() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        let bid = LimitOrder::new("28800538 A b B 44.26 100".split(' ').collect());
        ob.add(bid);
        let ro = ReduceOrder::new("28800744 R b 20".split(' ').collect());
        ob.reduce_order(&ro);
        let bid2 = LimitOrder::new("28800986 A c B 44.07 500".split(' ').collect());
        ob.add(bid2);
        let ret = ob.summarise_target();
        assert_eq!(ob.bids_total_size, 580);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800986);
        assert!(ob.cache.contains_key(&hash("b")));
        assert!(ob.cache.contains_key(&hash("c")));
        assert_eq!(ret, Some(Amount::new_from_str("8829.20")));
    }

    #[test]
    fn run_through_basic() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        ob.process("28800538 A b S 44.26 100");
        assert_eq!(ob.asks_total_size, 100);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, 28800538);
        assert!(ob.cache.contains_key(&hash("b")));
        assert_eq!(ob.summarise_target(), None);

        ob.process("28800562 A c B 44.10 100");
        assert_eq!(ob.asks_total_size, 100);
        assert_eq!(ob.bids_total_size, 100);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800562);
        assert!(ob.cache.contains_key(&hash("b")));
        assert!(ob.cache.contains_key(&hash("c")));
        assert_eq!(ob.summarise_target(), None);

        ob.process("28800744 R b 100");
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.bids_total_size, 100);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, 28800744);
        assert!(ob.cache.contains_key(&hash("b")));
        assert!(ob.cache.contains_key(&hash("c")));
        assert_eq!(ob.summarise_target(), None);

        ob.process("28800758 A d B 44.18 157");
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.bids_total_size, 257);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800758);
        assert!(ob.cache.contains_key(&hash("b")));
        assert!(ob.cache.contains_key(&hash("c")));
        assert!(ob.cache.contains_key(&hash("d")));
        assert_eq!(ob.summarise_target(), Some(Amount::new_from_str("8832.56")));

        ob.process("28800796 R d 157");
    }

    #[test]
    fn prices_vec_with_capacity() {
        let vec: AsksVec = AsksVec::with_capacity(10);
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn prices_vec_len() {
        let vec: AsksVec = AsksVec::with_capacity(10);
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn prices_vec_add() {
        let mut vec: AsksVec = AsksVec::with_capacity(10);
        vec.push(Amount::new());
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], Amount::new());
    }

    #[test]
    fn prices_vec_search_ok() {
        let mut prices_vec: AsksVec = AsksVec::with_capacity(10);
        let amounts_vec = ["44.10", "44.20", "60.00", "70.00"];
        for item in amounts_vec.iter() {
            let am_item = Amount::new_from_str(item);
            prices_vec.push(am_item);
        }
        let idx = prices_vec.binary_search(&Amount::new_from_str(&"44.20"));
        assert_eq!(idx, Ok(1));
    }

    #[test]
    fn prices_vec_search_err() {
        let mut prices_vec: AsksVec = AsksVec::with_capacity(10);
        let amounts_vec = ["44.10", "44.20", "60.00", "70.00"];
        for item in amounts_vec.iter() {
            let am_item = Amount::new_from_str(item);
            prices_vec.push(am_item);
        }
        let idx = prices_vec.binary_search(&Amount::new_from_str(&"84.20"));
        assert_eq!(idx, Err(4));
    }

}
