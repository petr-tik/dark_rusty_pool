extern crate fnv;

use std::collections::HashMap;
use std::env;
use std::io;
use std::io::prelude::*;

mod amount;
use amount::Amount;

mod bidamount;

mod orderside;
use orderside::OrderSide;

mod orders;

mod orderbook;
use orderbook::{IdPriceCacheFnvMap, OrderBook};

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
                )
                .expect("cannot lock");
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
            )
            .expect("cannot lock");
        } else if cur.is_none() && prev.is_some() {
            writeln!(
                stdout.lock(),
                "{} {} NA",
                ob.last_action_timestamp,
                !ob.last_action_side
            )
            .expect("cannot lock");
        }
        *prev = cur;
    }
}
