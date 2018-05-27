use std::collections::HashMap;
use std::collections::LinkedList;

#[derive(Copy, Clone)]
enum OrderSide {
    Bid, // buy 1
    Ask, // sell -1 like buying -100
}

struct ReduceOrder {
    // "28800744 R b 20"
    timestamp: String,
    id: String,
    size: i64,
}

impl ReduceOrder {
    fn new(input_line: String) -> Self {
        let input_vec: Vec<&str> = input_line.trim().split(" ").collect();
        let reduce_order = ReduceOrder {
            timestamp: input_vec[0].to_string(),
            id: input_vec[2].to_string(),
            size: input_vec[3].parse::<i64>().unwrap_or(0),
        };
        return reduce_order;
    }
}

struct LimitOrder {
    // "28800538 A b S 44.26 100"
    timestamp: String,
    id: String,
    side: OrderSide,
    price: i64,
    size: i64,
}

impl LimitOrder {
    fn new(input_line: String) -> Self {
        println!("{}", input_line);
        let input_vec: Vec<&str> = input_line.trim().split(" ").collect();

        let float_from_input = input_vec[4].parse::<f64>().unwrap_or(0.0) * 100.0;
        let side_factor: i64 = if input_vec[3].to_string() == "B" {
            1
        } else {
            -1
        };
        let addorder = LimitOrder {
            timestamp: input_vec[0].to_string(),
            id: input_vec[2].to_string(),
            side: match input_vec[3] {
                "B" => OrderSide::Bid,
                "S" => OrderSide::Ask,
                _ => OrderSide::Bid,
            },
            price: float_from_input.round() as i64,
            size: input_vec[5].parse::<i64>().unwrap_or(0) * side_factor,
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
    cache: HashMap<String, i64>,
}

impl IdPriceCache {
    fn new() -> Self {
        IdPriceCache {
            cache: HashMap::new(),
        }
    }

    fn insert(&mut self, order: &LimitOrder) {
        self.cache.insert(order.id.clone(), order.price);
    }
}

struct OrdersAtPrice {
    orders: LinkedList<LimitOrder>,
}

impl OrdersAtPrice {
    fn new() -> Self {
        OrdersAtPrice {
            orders: LinkedList::new(),
        }
    }

    fn reduce(&mut self, order: &ReduceOrder) {
        let mut order_iter = self.orders.iter_mut();
        while let Some(cur) = order_iter.next() {
            if cur.id != order.id {
                println!("Wrong id - next");
                order_iter.next();
            } else {
                if order.size >= cur.size {
                    cur.size = 0;
                } else {
                    cur.size -= order.size;
                }
                break;
            }
        }
    }

    fn insert(&mut self, order: &LimitOrder) {
        /* 
        Insert a new order into the list. 

        3 cases:
        1. empty list - insert the amount into list
        2. list with same orders (only buys or only sells, new order - the same) - insert at the back
        3. new order can cross some orders from the list

        */
        let order_amount = order.size;
        let mut order_amount_left: i64 = order_amount;
        if self.orders.is_empty()
            || (self.orders.front().unwrap().size.is_positive() == order_amount.is_positive()
                || self.orders.front().unwrap().size.is_negative() == order_amount.is_negative())
        {
            self.orders
                .push_back(LimitOrder::new_from(order, order.size));
        } else {
            let mut order_iter = self.orders.iter_mut();
            while let Some(cur) = order_iter.next() {
                println!("Order: size {}", cur.size);
                if order_amount_left.abs() == cur.size.abs() {
                    cur.size = 0;
                    order_amount_left = 0;
                    break;
                } else if order_amount_left.abs() > cur.size.abs() {
                    order_amount_left -= cur.size.abs();
                    cur.size = 0;
                    order_iter.next();
                } else {
                    // order_amount_left.abs() < cur.unwrap().abs()
                    cur.size -= order_amount_left;
                    order_amount_left = 0;
                    break;
                }
            }

            // or insert_next
        }
        self.orders
            .push_back(LimitOrder::new_from(&order, order_amount_left));
    }
}

struct OrderBook {
    cache: IdPriceCache,
    orders_at_price: HashMap<i64, OrdersAtPrice>,
}

impl OrderBook {
    fn new() -> Self {
        OrderBook {
            cache: IdPriceCache::new(),
            orders_at_price: HashMap::new(),
        }
    }

    fn add(&mut self, order: LimitOrder) {
        if self.orders_at_price.contains_key(&order.price) == false {
            self.orders_at_price
                .insert(order.price, OrdersAtPrice::new());
        }
        let orders_at_given_price: &mut OrdersAtPrice =
            self.orders_at_price.get_mut(&order.price).unwrap();
        orders_at_given_price.insert(&order);
        self.cache.insert(&order);
    }

    fn reduce_order(&mut self, ord: ReduceOrder) {
        /*

        First look up the price by the order id, then find the bucket by that price and find the order by id and reduce its amount

         */
        println!("Reducing orders");
        let price = self.cache.cache.get(&ord.id);
        match price {
            None => (),
            Some(price) => self.orders_at_price.get_mut(price).unwrap().reduce(&ord),
        }
    }
}

fn main() {
    println!("Hello, world!");
    let mut ob: OrderBook = OrderBook::new();
    let add_orders = [
        "28800538 A b S 44.26 200",
        "28800638 A c B 44.26 90",
        "28800738 A d B 44.26 20",
        "28800538 A f S 44.26 200",
    ];
    for order in add_orders.iter() {
        ob.add(LimitOrder::new(order.to_string()));
    }
    let reduce_orders = ["28800744 R f 20"];
    for ord in reduce_orders.iter() {
        ob.reduce_order(ReduceOrder::new(ord.to_string()));
    }
}
