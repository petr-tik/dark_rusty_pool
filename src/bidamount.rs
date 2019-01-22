use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result};
use std::ops::{AddAssign, Mul, MulAssign};

use amount::Amount;

// run unit tests with
// cargo test -- amount

#[derive(Copy, Clone, Debug, Eq)] // allows us to use BidAmount as a HashMap key
pub struct BidAmount {
    pub as_int: u64,
}

impl BidAmount {
    pub fn new() -> Self {
        BidAmount { as_int: 0 }
    }

    #[cfg(test)]
    fn new_from_str(input_string: &str) -> Self {
        let float_from_input = input_string.parse::<f64>();
        let float_res = match float_from_input {
            Ok(number_to_round) => number_to_round,
            Err(err) => panic!("Input string {} doesn't parse as f64 {}", input_string, err),
        };
        let float_times_hundred = float_res * 100.0;
        let int_res = float_times_hundred.round() as u64;
        BidAmount { as_int: int_res }
    }
}

impl Ord for BidAmount {
    fn cmp(&self, other: &BidAmount) -> Ordering {
        match self.as_int.cmp(&other.as_int) {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            Ordering::Equal => Ordering::Equal,
        }
    }
}

impl PartialOrd for BidAmount {
    fn partial_cmp(&self, other: &BidAmount) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BidAmount {
    fn eq(&self, other: &BidAmount) -> bool {
        self.as_int == other.as_int
    }
}

impl AddAssign for BidAmount {
    fn add_assign(&mut self, other_amount: Self) {
        self.as_int += other_amount.as_int;
    }
}

impl MulAssign<u64> for BidAmount {
    fn mul_assign(&mut self, multiplier: u64) {
        self.as_int *= multiplier;
    }
}

impl Mul<u64> for BidAmount {
    type Output = Self;
    fn mul(self, rhs: u64) -> Self {
        BidAmount {
            as_int: self.as_int * rhs,
        }
    }
}

impl Display for BidAmount {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let repr = self.as_int.to_string();
        let decimal_points = 2;
        let idx = repr.len() - decimal_points;
        let (quot_x, rem_x) = repr.split_at(idx);
        write!(f, "{}.{}", quot_x, rem_x)
    }
}

impl From<Amount> for BidAmount {
    fn from(item: Amount) -> Self {
        BidAmount {
            as_int: item.as_int,
        }
    }
}

impl<'a> From<&'a Amount> for BidAmount {
    fn from(item: &'a Amount) -> Self {
        BidAmount {
            as_int: item.as_int,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_from_str_works() {
        let am = BidAmount::new_from_str(&"44.12");
        assert_eq!(am.as_int, 4412);
    }

    #[test]
    fn constructor_default_works() {
        let am = BidAmount::new();
        assert_eq!(am.as_int, 0);
    }

    #[test]
    #[should_panic]
    fn bad_constructor_panics() {
        BidAmount::new_from_str(&"asda");
    }

    #[test]
    fn multiply_by_zero() {
        let mut am = BidAmount::new_from_str(&"44.12");
        am *= 0;
        assert_eq!(am.as_int, 0);
    }

    #[test]
    fn multiply_by_ten() {
        let mut am = BidAmount::new_from_str(&"44.12");
        am *= 10;
        assert_eq!(am.as_int, 44120);
    }

    #[test]
    fn add_two_amounts() {
        let mut am1 = BidAmount::new_from_str(&"44.12");
        let am2 = BidAmount::new_from_str(&"45.80");
        am1 += am2;
        assert_eq!(am1.as_int, 8992);
    }

    #[test]
    fn display_works() {
        use std::fmt::Write as FmtWrite;
        let input_string = "44.12";
        let am1 = BidAmount::new_from_str(&input_string);
        let mut res = String::new();
        write!(&mut res, "{}", am1).unwrap();
        assert_eq!(res, input_string);
    }

    #[test]
    fn compare_equals() {
        let input_string = "44.12";
        let am1 = BidAmount::new_from_str(&input_string);
        let am2 = BidAmount::new_from_str(&input_string);
        assert_eq!(am1, am2);
    }
    #[test]
    fn compare_less() {
        let am1 = BidAmount::new_from_str("44.12");
        let am2 = BidAmount::new_from_str("44.10");
        assert_eq!(am1.cmp(&am2), Ordering::Less);
    }

    #[test]
    fn compare_greater() {
        let am1 = BidAmount::new_from_str("44.12");
        let am2 = BidAmount::new_from_str("44.10");
        assert_eq!(am2.cmp(&am1), Ordering::Greater);
    }

    #[test]
    fn bin_search_with_new_order() {
        let v: Vec<BidAmount> = vec!["50.20", "49.00", "45.00", "41.00"]
            .into_iter()
            .map(|price| BidAmount::new_from_str(price))
            .collect();
        assert_eq!(v.binary_search(&BidAmount::new()), Err(4));
        assert_eq!(v.binary_search(&BidAmount::new_from_str("49.00")), Ok(1));
        assert_eq!(v.binary_search(&BidAmount::new_from_str("44.99")), Err(3));
    }

    #[test]
    fn convert_from_amount_to_bidamount() {
        let a = Amount::new();
        let ba: BidAmount = a.into();
        assert_eq!(ba, BidAmount::new());
    }

    #[test]
    fn convert_from_ref_amount_to_bidamount() {
        let a = Amount::new();
        let ba: &BidAmount = &a.into();
        assert_eq!(ba, &BidAmount::new());
    }

}
