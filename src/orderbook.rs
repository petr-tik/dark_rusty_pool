use std::cmp::min;

use amount::Amount;

use bidamount::BidAmount;

use orderside::OrderSide;

use orders::{LimitOrder, ReduceOrder};

/// Price cache strategy (for benchmarking)
pub trait IdPriceCache {
    fn insert(&mut self, order: &LimitOrder);
    fn contains_key(&self, key: &u64) -> bool;
    fn get(&self, key: &u64) -> Option<&(Amount, OrderSide)>;
}

pub type IdPriceCacheFnvMap = fnv::FnvHashMap<u64, (Amount, OrderSide)>;
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

type BidsVec = Vec<(BidAmount, Depth)>;
type AsksVec = Vec<(Amount, Depth)>;

pub struct OrderBook<T: IdPriceCache + Sized> {
    cache: T,
    bids_total_size: i64,
    asks: AsksVec,
    bids: BidsVec,
    asks_total_size: i64,
    target_size: i64,
    // only 1 side is affected on Reduce or Limit order
    pub last_action_side: OrderSide, // which side was touched last
    pub last_action_timestamp: i64,  // timestamp of last touched side
}

impl<T: IdPriceCache + Sized> OrderBook<T> {
    pub fn new(target_size: i64, cache: T) -> Self {
        let cap = 256;
        OrderBook {
            cache,
            asks: AsksVec::with_capacity(cap),
            bids: BidsVec::with_capacity(cap),
            bids_total_size: 0,
            asks_total_size: 0,
            target_size,
            last_action_side: OrderSide::Ask,
            last_action_timestamp: 000_000_000,
        }
    }

    fn _add_to_asks(&mut self, order: &LimitOrder) {
        match self
            .asks
            .binary_search_by_key(&order.price, |&(price, _size)| price)
        {
            Ok(idx) => {
                self.asks[idx].1 += order.size;
            }
            Err(idx) => {
                self.asks.insert(idx, (order.price, order.size));
            }
        }
        self.asks_total_size += order.size;
    }

    fn _add_to_bids(&mut self, order: &LimitOrder) {
        match self
            .bids
            .binary_search_by_key(&order.price.into(), |&(price, _size)| price)
        {
            Ok(idx) => {
                self.bids[idx].1 += order.size;
            }
            Err(idx) => {
                self.bids.insert(idx, (order.price.into(), order.size));
            }
        }
        self.bids_total_size += order.size;
    }

    pub fn add(&mut self, order: LimitOrder) {
        if order.side == OrderSide::Bid {
            self._add_to_bids(&order);
        } else if order.side == OrderSide::Ask {
            self._add_to_asks(&order);
        }
        self.cache.insert(&order);
        self.last_action_timestamp = order.timestamp;
        self.last_action_side = order.side;
    }

    pub fn reduce_order(&mut self, order: &ReduceOrder) {
        let (price, side) = match self.cache.get(&order.id) {
            Some(tup) => (tup),
            None => panic!("No order under key {}", &order.id),
        };
        if side == &OrderSide::Ask {
            if let Ok(idx) = self
                .asks
                .binary_search_by_key(price, |&(price, _size)| price)
            {
                self.asks[idx].1 -= &order.size;
                self.asks_total_size -= order.size;
            }
        } else if side == &OrderSide::Bid {
            if let Ok(idx) = self
                .bids
                .binary_search_by_key(&price.into(), |&(price, _size)| price)
            {
                self.bids[idx].1 -= &order.size;
                self.bids_total_size -= order.size;
            }
        }
        self.last_action_timestamp = order.timestamp;
        self.last_action_side = *side;
    }

    pub fn summarise_target(&self) -> Option<Amount> {
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
        for (price, depth) in self.asks.iter() {
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
        let mut res = BidAmount::new();
        let mut target_left = self.target_size;
        for (price, depth) in self.bids.iter() {
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
        res.into()
    }

    pub fn process(&mut self, instruction: &str) {
        let order_vec: Vec<&str> = instruction.trim().split(' ').collect();
        if order_vec[1] == "A" {
            self.add(LimitOrder::new(&order_vec));
        } else if order_vec[1] == "R" {
            self.reduce_order(&ReduceOrder::new(&order_vec));
        } else {
            eprintln!("Error processing {}", instruction);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orders::hash;

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

        ob.process("28800538 A b S 44.26 100");
        assert_eq!(ob.asks_total_size, 100);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.summarise_target(), None);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, 28800538);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert_eq!(
            ob.asks
                .binary_search_by_key(&price, |&(price, _size)| price),
            Ok(0)
        );
    }

    #[test]
    fn orderbook_add_bid() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());

        ob.process("28800538 A b B 44.26 100");
        assert_eq!(ob.bids_total_size, 100);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.summarise_target(), None);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800538);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        let idx = ob.bids.binary_search_by_key(&price.into(), |&(p, _s)| p);
        assert_eq!(idx, Ok(0));
    }

    #[test]
    fn orderbook_reduce_ask() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());
        ob.process("28800538 A b S 44.26 100");
        ob.process("28800744 R b 20");
        assert_eq!(ob.asks_total_size, 80);
        assert_eq!(ob.bids_total_size, 0);
        assert_eq!(ob.summarise_target(), None);
        assert_eq!(ob.last_action_side, OrderSide::Ask);
        assert_eq!(ob.last_action_timestamp, 28800744);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert_eq!(
            ob.asks
                .binary_search_by_key(&price, |&(price, _size)| price),
            Ok(0)
        );
    }

    #[test]
    fn orderbook_reduce_bid() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());

        ob.process("28800538 A b B 44.26 100");
        ob.process("28800744 R b 20");
        assert_eq!(ob.bids_total_size, 80);
        assert_eq!(ob.asks_total_size, 0);
        assert_eq!(ob.last_action_side, OrderSide::Bid);
        assert_eq!(ob.last_action_timestamp, 28800744);
        assert_eq!(ob.summarise_target(), None);
        assert!(ob.cache.contains_key(&hash("b")));
        let price = Amount::new_from_str("44.26");
        assert_eq!(
            ob.bids
                .binary_search_by_key(&price, |&(price, _size)| price.into()),
            Ok(0)
        );
    }

    #[test]
    fn orderbook_add_reduce_add() {
        let target_size = 200;
        let mut ob = OrderBook::new(target_size, IdPriceCacheFnvMap::default());

        ob.process("28800538 A b B 44.26 100");
        ob.process("28800744 R b 20");
        ob.process("28800986 A c B 44.07 500");
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
    fn asks_vec_add() {
        let mut vec: AsksVec = AsksVec::with_capacity(10);
        vec.push((Amount::new(), 0));
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], (Amount::new(), 0));
    }

    #[test]
    fn asks_vec_search_ok() {
        let mut asks_vec: AsksVec = AsksVec::with_capacity(10);
        let amounts_vec = ["44.10", "44.20", "60.00", "70.00"];
        for item in amounts_vec.iter() {
            let am_item = Amount::new_from_str(item);
            asks_vec.push((am_item, 100));
        }
        let idx =
            asks_vec.binary_search_by_key(&Amount::new_from_str(&"44.20"), |&(price, _size)| price);
        assert_eq!(idx, Ok(1));
    }

    #[test]
    fn asks_vec_search_err() {
        let mut asks_vec: AsksVec = AsksVec::with_capacity(10);
        let amounts_vec = ["44.10", "44.20", "60.00", "70.00"];
        let size = 100;
        for item in amounts_vec.iter() {
            let am_item = Amount::new_from_str(item);
            asks_vec.push((am_item, size));
        }
        let idx = asks_vec.binary_search(&(Amount::new_from_str(&"84.20"), size));
        assert_eq!(idx, Err(4));
    }

}
