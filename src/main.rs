use std::cmp::min;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::LinkedList;

#[derive(Copy, Clone, Debug, PartialEq)]
enum OrderSide {
    Bid, // buy
    Ask, // sell
}

#[derive(Debug)]
struct ReduceOrder {
    // "28800744 R b 20"
    timestamp: String,
    id: String,
    size: i64,
}

impl ReduceOrder {
    fn new(input_line: &str) -> Self {
        let input_vec: Vec<&str> = input_line.trim().split(" ").collect();
        let reduce_order = ReduceOrder {
            timestamp: input_vec[0].to_string(),
            id: input_vec[2].to_string(),
            size: input_vec[3].parse::<i64>().unwrap_or(0),
        };
        return reduce_order;
    }
}

#[derive(Clone, Debug)]
struct LimitOrder {
    // "28800538 A b S 44.26 100"
    timestamp: String,
    id: String,
    side: OrderSide,
    price: i64,
    size: i64,
}

impl LimitOrder {
    fn new(input_line: &str) -> Self {
        println!("{}", input_line);
        let input_vec: Vec<&str> = input_line.trim().split(" ").collect();

        let float_from_input = input_vec[4].parse::<f64>().unwrap_or(0.0) * 100.0;
        let addorder = LimitOrder {
            timestamp: input_vec[0].to_string(),
            id: input_vec[2].to_string(),
            side: match input_vec[3] {
                "B" => OrderSide::Bid,
                "S" => OrderSide::Ask,
                _ => OrderSide::Bid,
            },
            price: float_from_input.round() as i64,
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
    cache: HashMap<String, (i64, OrderSide)>,
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
                println!("Found the order to reduce");
                if order.size >= cur.size {
                    cur.size = 0;
                } else {
                    cur.size -= order.size;
                }
                println!("{:?}", cur);
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

    fn amount_earned_or_spent_for(&self, target_size: i64) -> i64 {
        return self.depth * target_size;
    }
}

struct OrderBook {
    cache: IdPriceCache,
    bids_at_price: BTreeMap<i64, OrdersAtPrice>,
    bids_total_size: i64,
    asks_at_price: BTreeMap<i64, OrdersAtPrice>,
    asks_total_size: i64,
}

impl OrderBook {
    fn new() -> Self {
        OrderBook {
            cache: IdPriceCache::new(),
            bids_at_price: BTreeMap::new(),
            asks_at_price: BTreeMap::new(),
            bids_total_size: 0,
            asks_total_size: 0,
        }
    }

    fn add(&mut self, order: LimitOrder) {
        if order.side == OrderSide::Bid {
            if self.bids_at_price.contains_key(&order.price) == false {
                self.bids_at_price.insert(order.price, OrdersAtPrice::new());
            }
            let orders_at_given_price: &mut OrdersAtPrice =
                self.bids_at_price.get_mut(&order.price).unwrap();
            orders_at_given_price.insert(&order);
            self.bids_total_size += order.size;
        } else {
            if self.asks_at_price.contains_key(&order.price) == false {
                self.asks_at_price.insert(order.price, OrdersAtPrice::new());
            }
            let orders_at_given_price: &mut OrdersAtPrice =
                self.asks_at_price.get_mut(&order.price).unwrap();
            orders_at_given_price.insert(&order);
            self.asks_total_size += order.size;
        }
        self.cache.insert(&order);
    }

    fn reduce_order(&mut self, order: ReduceOrder) {
        /*

        First look up the price and side by the order id,
        then find the bucket by that price and side 
        
        call the reduce method of the orders_at_price

         */

        let (price, side) = self.cache.cache.get(&order.id).unwrap();
        if side == &OrderSide::Ask {
            self.asks_at_price.get_mut(&price).unwrap().reduce(&order);
            self.asks_total_size -= order.size;
        } else if side == &OrderSide::Bid {
            self.bids_at_price.get_mut(&price).unwrap().reduce(&order);
            self.bids_total_size -= order.size;
        }
    }

    fn summarise_target(&self, target: i64) {
        if target <= self.bids_total_size {
            let mut target_left = target;
            for (price, bucket) in self.bids_at_price.iter() {
                if target_left <= 0 {
                    break;
                }
                let res = price * min(bucket.depth, target_left);
                target_left -= bucket.depth;
                println!("Bought for {} at price {}", res, price);
            }
        }
        if target <= self.asks_total_size {
            for (price, bucket) in self.asks_at_price.iter().rev() {
                let res = price * bucket.depth;
                println!("Sold for {} at price {}", res, price);
            }
        }
    }
}

fn main() {
    println!("Hello, world!");
    let mut ob: OrderBook = OrderBook::new();
    use std::io;
    use std::io::prelude::*;
    let stdin = io::stdin();
    for order_line in stdin.lock().lines() {
        let unwrapped_line: &str = &order_line.unwrap();
        let order_vec: Vec<&str> = unwrapped_line.trim().split(" ").collect();
        if order_vec[1] == "A" {
            ob.add(LimitOrder::new(unwrapped_line));
        } else if order_vec[1] == "R" {
            ob.reduce_order(ReduceOrder::new(unwrapped_line));
        }
    }
}
