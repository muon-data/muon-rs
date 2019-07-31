// parse.rs
//
// Copyright (c) 2019  Douglas Lau
//
use std::str::FromStr;

/// Integer trait
pub trait Integer: FromStr {
    fn from_str_radix(src: &str, radix: u32) -> Option<Self>;
}

macro_rules! impl_integer {
    () => {};
    ($i:ident $($more:ident)*) => {
        impl Integer for $i {
            fn from_str_radix(src: &str, radix: u32) -> Option<Self> {
                Self::from_str_radix(src, radix).ok()
            }
        }
        impl_integer!($($more)*);
    };
}

impl_integer!(i8 i16 i32 i64 i128 u8 u16 u32 u64 u128);

/// Marker trait for float types
pub trait Float: FromStr {}

impl Float for f32 {}
impl Float for f64 {}

/// Parse an integer from a string slice
pub fn int<T: Integer>(v: &str) -> Option<T> {
    T::from_str(v).ok().or_else(|| int_fallback(v))
}

/// Fallback for integer parsing
fn int_fallback<T: Integer>(v: &str) -> Option<T> {
    if v.starts_with("0b_") {
        T::from_str_radix(&sanitize_num(&v[3..], 2), 2)
    } else if v.starts_with("0b") {
        T::from_str_radix(&sanitize_num(&v[2..], 2), 2)
    } else if v.starts_with("0x_") {
        T::from_str_radix(&sanitize_num(&v[3..], 16), 16)
    } else if v.starts_with("0x") {
        T::from_str_radix(&sanitize_num(&v[2..], 16), 16)
    } else {
        T::from_str(&sanitize_num(v, 10)).ok()
    }
}

/// Parse a float from a string slice
pub fn float<T: Float>(v: &str) -> Option<T> {
    T::from_str(v)
        .ok()
        .or_else(|| T::from_str(&sanitize_num(v, 10)).ok())
}

/// Sanitize a number, removing valid underscores
fn sanitize_num(value: &str, radix: u32) -> String {
    let mut val = String::with_capacity(value.len());
    for v in value.split('_') {
        // Check character before underscore is a decimal digit
        if let Some(before) = val.as_bytes().last() {
            if !char::from(*before).is_digit(radix) {
                val.push('_')
            }
        }
        // Check character after underscore is a decimal digit
        if let Some(after) = v.as_bytes().first() {
            if *after != b'-' && !char::from(*after).is_digit(radix) {
                val.push('_')
            }
        } else {
            val.push('_')
        }
        val.push_str(v);
    }
    val
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ints() {
        assert_eq!(int("0"), Some(0));
        assert_eq!(int("25"), Some(25));
        assert_eq!(int("-42"), Some(-42));
        assert_eq!(int("+15"), Some(15));
        assert_eq!(int("0b101010"), Some(42u8));
        assert_eq!(int::<u32>("0o755"), None);
        assert_eq!(int("0x1Ac"), Some(428));
        assert_eq!(int("0xffff"), Some(0xFFFFu16));
        assert_eq!(int("0x1234567890"), Some(0x1234567890i64));
        assert_eq!(int("0x1000000000000000"), Some(0x1000000000000000u64));
        assert_eq!(int("0x10000000000000000"), Some(0x10000000000000000u128));
        assert_eq!(int("1_234_567_890"), Some(1234567890));
        assert_eq!(int("-12_34_56"), Some(-123456));
        assert_eq!(int("0b_1111_0000_1111"), Some(0xF0F));
        assert_eq!(int("0x123_FED"), Some(0x123_FED));
        assert_eq!(int::<u8>("0.0"), None);
        assert_eq!(int::<u8>("255"), Some(255));
        assert_eq!(int::<u8>("256"), None);
        assert_eq!(int::<u8>("-1"), None);
        assert_eq!(int::<i8>("-128"), Some(-128));
        assert_eq!(int::<i8>("127"), Some(127));
        assert_eq!(int::<i8>("-129"), None);
        assert_eq!(int::<i8>("128"), None);
        assert_eq!(int::<i16>("+-0"), None);
        // assert_eq!(int::<u32>("00"), None);
        assert_eq!(int::<u32>("abc"), None);
        assert_eq!(int::<i32>("0b0000_"), None);
        assert_eq!(int::<i32>("0b0000__0000"), None);
    }

    #[test]
    fn floats() {
        assert_eq!(float::<f32>("+3.14159").unwrap(), 3.14159);
        assert_eq!(float::<f32>("-0.0").unwrap(), -0.0);
        assert_eq!(float::<f32>("1e15").unwrap(), 1e15);
        assert_eq!(float::<f32>("0.5431e-28").unwrap(), 0.5431e-28);
        assert_eq!(float::<f32>(".123456").unwrap(), 0.123456);
        assert_eq!(float::<f32>("0.1e1_2").unwrap(), 0.1e12);
        assert_eq!(float::<f32>("8_765.432_1").unwrap(), 8_765.432_1);
        assert_eq!(float::<f32>("100").unwrap(), 100.0);
        assert!(float::<f32>("123_.456").is_none());
        assert!(float::<f32>("_123.456").is_none());
        assert!(float::<f32>("123.456_").is_none());
        assert!(float::<f32>("123._456").is_none());
        assert!(float::<f32>("12.34.56").is_none());
        assert_eq!(float::<f32>("NaN").unwrap().to_string(), "NaN");
        assert_eq!(float::<f64>("-123.456789e0").unwrap(), -123.456789);
        assert_eq!(float::<f64>("inf").unwrap(), std::f64::INFINITY);
        assert_eq!(float::<f64>("-inf").unwrap(), std::f64::NEG_INFINITY);
        assert!(float::<f64>("1__0.0").is_none());
        assert!(float::<f64>("infinity").is_none());
        assert!(float::<f64>("INF").is_none());
        assert!(float::<f64>("nan").is_none());
        assert!(float::<f64>("nAn").is_none());
    }
}
