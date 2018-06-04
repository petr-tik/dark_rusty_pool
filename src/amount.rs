pub mod amount {

    use std::fmt::Display;
    use std::fmt::Formatter;
    use std::fmt::Result;
    use std::num::ParseFloatError;
    use std::ops::AddAssign;
    use std::ops::MulAssign;

    // run unit tests with
    // cargo test -- amount

    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)] // allows us to use Amount as a HashMap key
    pub struct Amount {
        pub as_int: u32,
    }

    impl Amount {
        pub fn new() -> Self {
            return Amount { as_int: 0 };
        }

        pub fn new_from_str(input_string: &str) -> Self {
            let float_from_input = input_string.parse::<f32>();
            let float_res = match float_from_input {
                Ok(number_to_round) => number_to_round,
                Err(ParseFloatError) => {
                    panic!("Input string {} doesn't parse as f32", input_string)
                }
            };
            let float_times_hundred = float_res * 100.0;
            let int_res = float_times_hundred.round() as u32;
            return Amount { as_int: int_res };
        }
    }

    impl AddAssign for Amount {
        fn add_assign(&mut self, other_amount: Self) {
            self.as_int += other_amount.as_int;
        }
    }

    impl MulAssign<u32> for Amount {
        fn mul_assign(&mut self, multiplier: u32) {
            self.as_int *= multiplier;
        }
    }

    impl Display for Amount {
        fn fmt(&self, f: &mut Formatter) -> Result {
            let quot_x = self.as_int.checked_div(100).unwrap();
            let rem_x = self.as_int.checked_rem(100).unwrap();

            write!(f, "{}.{}", quot_x, rem_x)
        }
    }
}

// todo - turn price into unsigned value

#[cfg(test)]
mod tests {
    use amount::amount::*;
    #[test]
    fn constructor_from_str_works() {
        let am = Amount::new_from_str(&"44.12");
        assert_eq!(am.as_int, 4412);
    }

    #[test]
    fn constructor_default_works() {
        let am = Amount::new();
        assert_eq!(am.as_int, 0);
    }

    #[test]
    #[should_panic]
    fn bad_constructor_panics() {
        Amount::new_from_str(&"asda");
    }

    #[test]
    fn multiply_by_zero() {
        let mut am = Amount::new_from_str(&"44.12");
        am *= 0;
        assert_eq!(am.as_int, 0);
    }

    #[test]
    fn multiply_by_ten() {
        let mut am = Amount::new_from_str(&"44.12");
        am *= 10;
        assert_eq!(am.as_int, 44120);
    }

    #[test]
    fn add_two_amounts() {
        let mut am1 = Amount::new_from_str(&"44.12");
        let am2 = Amount::new_from_str(&"45.80");
        am1 += am2;
        assert_eq!(am1.as_int, 8992);
    }

    #[test]
    fn display_works() {
        use std::fmt::Write as FmtWrite;
        let input_string = "44.12";
        let am1 = Amount::new_from_str(&input_string);
        let mut res = String::new();
        write!(&mut res, "{}", am1).unwrap();
        assert_eq!(res, input_string);
    }
}
