use std::cmp::min;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fmt::{Display, Formatter, Result};
use std::env;
use std::io;
use std::io::prelude::*;
use std::result::Result::{Ok, Err};

mod amount;
use amount::amount::Amount;


#[derive(Copy, Clone, Debug, PartialEq)]
enum OrderSide {
    Bid, // buy
    Ask, // sell
}

impl Display for OrderSide {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            OrderSide::Ask => write!(f, "S"),
            OrderSide::Bid => write!(f, "B"),
        }
    }
}

#[derive(Debug)]
struct ReduceOrder {
    // "28800744 R b 20"
    timestamp: String,
    id: String,
    size: i64,
}

impl ReduceOrder {
    pub fn new(input_vec: Vec<&str>) -> Self {
        let reduce_order = ReduceOrder {
            timestamp: input_vec[0].to_string(),
            id: input_vec[2].to_string(),
            size: input_vec[3].parse::<i64>().unwrap_or(0),
        };
        return reduce_order;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LimitOrder {
    // "28800538 A b S 44.26 100"
    pub timestamp: String,
    pub id: String,
    pub side: OrderSide,
    pub price: Amount,
    pub size: i64,
}

impl LimitOrder {
    pub fn new(input_vec: Vec<&str>) -> Self {
        let float_from_input = input_vec[4].parse::<f64>().unwrap_or(0.0) * 100.0;
        let addorder = LimitOrder {
            timestamp: input_vec[0].to_string(),
            id: input_vec[2].to_string(),
            side: match input_vec[3] {
                "B" => OrderSide::Bid,
                "S" => OrderSide::Ask,
                _ => panic!("Couldn't parse order side from {}", input_vec[3]),
            },
            price: Amount::new_from_str(&input_vec[4]),
            size: input_vec[5].parse::<i64>().unwrap_or(0),
        };
        return addorder;
    }

    fn new_from(old_order: &LimitOrder, new_size: i64) -> Self {
        let order = LimitOrder {
            timestamp: old_order.timestamp.clone(),
            id: old_order.id.clone(),
            side: old_order.side,
            price: old_order.price,
            size: new_size,
        };
        return order;
    }
}

// #[derive(Default)]
struct IdPriceCache {
    cache: HashMap<String, (Amount, OrderSide)>,
}

impl IdPriceCache {
    fn new() -> Self {
        IdPriceCache {
            cache: HashMap::new(),
        }
    }

    fn insert(&mut self, order: &LimitOrder) {
        self.cache
            .insert(order.id.clone(), (order.price, order.side));
    }
}

#[derive(Debug)]
struct OrdersAtPrice {
    orders: LinkedList<LimitOrder>,
    depth: i64,
}

impl OrdersAtPrice {
    fn new() -> Self {
        OrdersAtPrice {
            orders: LinkedList::new(),
            depth: 0,
        }
    }

    fn reduce(&mut self, order: &ReduceOrder) {
        let mut order_iter = self.orders.iter_mut();
        while let Some(cur) = order_iter.next() {
            if cur.id != order.id {
                continue;
            } else {
                // println!("Found the order to reduce");
                if order.size >= cur.size {
                    cur.size = 0;
                } else {
                    cur.size -= order.size;
                }
                // println!("{:?}", cur);
                self.depth -= order.size;
                break;
            }
        }
    }

    fn insert(&mut self, order: &LimitOrder) {
        /* 
        Insert a new order into the list. 
        */
        self.orders.push_back(order.clone());
        self.depth += order.size;
    }
}

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

impl OrderBook {
    fn new(target_size: i64) -> Self {
        OrderBook {
            cache: IdPriceCache::new(),
            bids_at_price: BTreeMap::new(),
            asks_at_price: BTreeMap::new(),
            bids_total_size: 0,
            asks_total_size: 0,
            target_size: target_size,
            last_action_side: OrderSide::Bid,
            last_action_timestamp: "dummy_string".to_string(),
        }
    }

    fn add(&mut self, order: LimitOrder) {
        if order.side == OrderSide::Bid {
            self.last_action_side = OrderSide::Bid;
            if self.bids_at_price.contains_key(&order.price) == false {
                self.bids_at_price.insert(order.price, OrdersAtPrice::new());
            }
            let orders_at_given_price: &mut OrdersAtPrice =
                self.bids_at_price.get_mut(&order.price).unwrap();
            orders_at_given_price.insert(&order);
            self.bids_total_size += order.size;
        } else if order.side == OrderSide::Ask {
            self.last_action_side = OrderSide::Ask;
            if self.asks_at_price.contains_key(&order.price) == false {
                self.asks_at_price.insert(order.price, OrdersAtPrice::new());
            }
            let orders_at_given_price: &mut OrdersAtPrice =
                self.asks_at_price.get_mut(&order.price).unwrap();
            orders_at_given_price.insert(&order);
            self.asks_total_size += order.size;
        }
        self.cache.insert(&order);
        self.last_action_timestamp = order.timestamp;
    }

    fn reduce_order(&mut self, order: ReduceOrder) {
        /*

        First look up the price and side by the order id,
        then find the bucket by that price and side 
        
        call the reduce method of the orders_at_price

         */

        let (price, side) = self.cache.cache.get(&order.id).unwrap();
        self.last_action_timestamp = order.timestamp.clone();
        if side == &OrderSide::Ask {
            self.asks_at_price.get_mut(&price).unwrap().reduce(&order);
            self.asks_total_size -= order.size;
            self.last_action_side == OrderSide::Ask;
        } else if side == &OrderSide::Bid {
            self.bids_at_price.get_mut(&price).unwrap().reduce(&order);
            self.bids_total_size -= order.size;
            self.last_action_side == OrderSide::Bid;
        }
    }

    fn summarise_target(&self) -> Option<Amount> {
        if self.bids_total_size >= self.target_size && self.last_action_side == OrderSide::Bid {
            return Some(self.summarise_amount_from_asks());

        } else if self.asks_total_size >= self.target_size
            && self.last_action_side == OrderSide::Ask
        {
            return Some(self.summarise_amount_from_bids());
        }
        None
    }

    fn summarise_amount_from_asks(&self) -> Amount {
        let mut res = Amount::new();
        let mut target_left = self.target_size;
        for (price, bucket) in self.asks_at_price.iter().rev() {
            if target_left <= 0 {
                break;
            }
            let available_in_this_bucket = min(bucket.depth, target_left);

            res += *price * available_in_this_bucket;
            target_left -= available_in_this_bucket;
            // println!("At price {} we sold {} and now have {} left to sell", *price, available_in_this_bucket, target_left);
        }
        res
    }

    fn summarise_amount_from_bids(&self) -> Amount {
        let mut res = Amount::new();
        let mut target_left = self.target_size;
        for (price, bucket) in self.bids_at_price.iter() {
            if target_left <= 0 {
                break;
            }
            let available_in_this_bucket = min(bucket.depth, target_left);
            res += *price * available_in_this_bucket;
            target_left -= available_in_this_bucket;
        }
        res
    }
}

fn get_target_size() -> i64 {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Need target size input");
    }
    let target_size: i64 = args[1].parse::<i64>().unwrap_or(0);
    
    target_size

}

fn main() {
    let target_size = get_target_size();
    let mut ob: OrderBook = OrderBook::new(target_size);

    let stdin = io::stdin();
    for order_line in stdin.lock().lines() {
        let unwrapped_line: &str = &order_line.unwrap();
        let order_vec: Vec<&str> = unwrapped_line.trim().split(" ").collect();
        if order_vec[1] == "A" {
            ob.add(LimitOrder::new(order_vec));
        } else if order_vec[1] == "R" {
            ob.reduce_order(ReduceOrder::new(order_vec));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_at_price_constructor() {
        let oap = OrdersAtPrice::new();
        assert_eq!(oap.depth, 0);
        assert!(oap.orders.is_empty());
    }

    #[test]
    fn orders_at_price_add_ask() {
        let mut oap = OrdersAtPrice::new();
        let lo = LimitOrder::new("28800538 A b S 44.26 100".split(" ").collect());
        oap.insert(&lo);
        assert_eq!(oap.depth, 100);
        assert_eq!(oap.orders.len(), 1);
        assert_eq!(oap.orders.front().unwrap(), &lo);
    }

    #[test]
    fn orders_at_price_add_and_reduce() {
        let mut oap = OrdersAtPrice::new();
        let lo = LimitOrder::new("28800538 A b S 44.26 100".split(" ").collect());
        oap.insert(&lo);
        let ro = ReduceOrder::new("28800744 R b 20".split(" ").collect());

        oap.reduce(&ro);
        let new_lo = LimitOrder::new_from(&lo, lo.size - ro.size);
        assert_eq!(oap.depth, lo.size - ro.size);
        assert_eq!(oap.orders.len(), 1);
        assert_eq!(oap.orders.front().unwrap(), &new_lo);
    }

    #[test]
    fn orderbook_constructor_works() {
        let target_size = 500;
        let ob = OrderBook::new(target_size);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.target_size, target_size);
        assert_eq!(ob.last_action_timestamp, "dummy_string");
        assert_eq!(ob.last_action_side, OrderSide::Bid);
    }

    #[test]
    fn orderbook_add_ask() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size);
        let ask = LimitOrder::new("28800538 A b S 44.26 100".split(" ").collect());
        ob.add(ask);
        assert_eq!(ob.asks_total_size, 100);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, "28800538");
        assert!(ob.cache.cache.contains_key("b"));
        let price = Amount::new_from_str("44.26");
        assert!(ob.asks_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_add_bid() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size);
        let bid = LimitOrder::new("28800538 A b B 44.26 100".split(" ").collect());
        ob.add(bid);
        assert_eq!(ob.bids_total_size, 100);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, "28800538");
        assert!(ob.cache.cache.contains_key("b"));
        let price = Amount::new_from_str("44.26");
        assert!(ob.bids_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_reduce_ask() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size);
        let ask = LimitOrder::new("28800538 A b S 44.26 100".split(" ").collect());
        ob.add(ask);
        let ro = ReduceOrder::new("28800744 R b 20".split(" ").collect());
        ob.reduce_order(ro);
        assert_eq!(ob.asks_total_size, 80);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, "28800744");
        assert!(ob.cache.cache.contains_key("b"));
        let price = Amount::new_from_str("44.26");
        assert!(ob.asks_at_price.contains_key(&price));
    }

    #[test]
    fn orderbook_reduce_bid() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size);
        let bid = LimitOrder::new("28800538 A b B 44.26 100".split(" ").collect());
        ob.add(bid);
        let ro = ReduceOrder::new("28800744 R b 20".split(" ").collect());
        ob.reduce_order(ro);
        assert_eq!(ob.bids_total_size, 80);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, "28800744");
        assert!(ob.cache.cache.contains_key("b"));
        let price = Amount::new_from_str("44.26");
        assert!(ob.bids_at_price.contains_key(&price));
    }

}
