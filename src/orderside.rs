use std::fmt::{Display, Formatter, Result};
use std::ops::Not;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OrderSide {
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

impl Not for OrderSide {
    type Output = OrderSide;

    fn not(self) -> OrderSide {
        match self {
            OrderSide::Bid => OrderSide::Ask,
            OrderSide::Ask => OrderSide::Bid,
        }
    }
}
