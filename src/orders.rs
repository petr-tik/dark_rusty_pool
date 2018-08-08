use std::hash::{Hash, Hasher};

use super::amount::Amount;
use super::orderside::OrderSide;

pub fn hash<T: Hash>(x: T) -> u64 {
    let mut hasher = fnv::FnvHasher::default();
    x.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
pub struct ReduceOrder {
    // "28800744 R b 20"
    pub timestamp: i64,
    pub id: u64,
    pub size: i64,
}

impl ReduceOrder {
    pub fn new(input_vec: Vec<&str>) -> Self {
        assert!(input_vec.len() <= 4);
        ReduceOrder {
            timestamp: input_vec[0].parse::<i64>().unwrap_or(0),
            id: hash(&input_vec[2]),
            size: input_vec[3].parse::<i64>().unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LimitOrder {
    // "28800538 A b S 44.26 100"
    pub timestamp: i64,
    pub id: u64,
    pub side: OrderSide,
    pub price: Amount,
    pub size: i64,
}

impl LimitOrder {
    pub fn new(input_vec: Vec<&str>) -> Self {
        assert!(input_vec.len() <= 6);
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
    fn reduce_order_constructor() {
        let ro = ReduceOrder::new("28800744 R b 20".split(' ').collect());
        assert_eq!(ro.timestamp, 28800744);
        assert_eq!(ro.size, 20);
        assert_eq!(ro.id, hash("b"));
    }

}
