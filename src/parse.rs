// parse.rs
//
// Copyright (c) 2019-2020  Douglas Lau
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

impl_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

/// Number trait
pub trait Number: FromStr + std::ops::Neg<Output = Self> {
    const INFINITY: Self;
    const NEG_INFINITY: Self;

    fn nan(sign: Sign) -> Self;
}

macro_rules! impl_number {
    () => {};
    (
        ($n:ident, $i:ident, $b:literal) $(,)?
        $(($more_n:ident, $more_i:ident, $more_b:literal))*
    ) => {
        impl Number for $n {
            const INFINITY: Self = $n::INFINITY;
            const NEG_INFINITY: Self = $n::NEG_INFINITY;

            fn nan(sign: Sign) -> Self {
                let sign_mask = !($i::from(sign == Sign::Positive) << $b);
                $n::from_bits($i::MAX & sign_mask)
            }
        }
        impl_number!($(($more_n, $more_i, $more_b))*);
    };
}

impl_number!((f32, u32, 31), (f64, u64, 63));

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
}

/// Parse an integer from a string slice
pub fn int<T: Integer>(v: &str) -> Option<T> {
    v.parse().ok().or_else(|| int_fallback(v))
}

/// Fallback for integer parsing
fn int_fallback<T: Integer>(v: &str) -> Option<T> {
    if let Some(binary) = v.strip_prefix('b') {
        T::from_str_radix(&sanitize_num(binary, 2), 2)
    } else if let Some(hexadecimal) = v.strip_prefix('x') {
        T::from_str_radix(&sanitize_num(hexadecimal, 16), 16)
    } else {
        sanitize_num(v, 10).parse().ok()
    }
}

/// Parse a number from a string slice
pub fn number<T: Number>(v: &str) -> Option<T> {
    let (v, sign) = if let Some(v) = v.strip_prefix('-') {
        (v, Sign::Negative)
    } else {
        (v.strip_prefix('+').unwrap_or(v), Sign::Positive)
    };
    let first = 'value: {
        return Some(match (v, sign) {
            ("inf", Sign::Negative) => T::NEG_INFINITY,
            ("inf", Sign::Positive) => T::INFINITY,
            ("NaN", sign) => T::nan(sign),
            _ => break 'value v.chars().next()?,
        });
    };

    (first.is_ascii_digit() || first == '.')
        .then(|| v.parse().ok().or_else(|| sanitize_num(v, 10).parse().ok()))?
        .map(|v: T| if sign == Sign::Negative { -v } else { v })
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
        assert_eq!(int("00"), Some(0));
        assert_eq!(int("005"), Some(5));
        assert_eq!(int("25"), Some(25));
        assert_eq!(int("-42"), Some(-42));
        assert_eq!(int("+15"), Some(15));
        assert_eq!(int("b101010"), Some(42u8));
        assert_eq!(int("x1Ac"), Some(428));
        assert_eq!(int("xffff"), Some(0xFFFFu16));
        assert_eq!(int("x1234567890"), Some(0x1234567890i64));
        assert_eq!(int("x1000000000000000"), Some(0x1000000000000000u64));
        assert_eq!(int("x10000000000000000"), Some(0x10000000000000000u128));
        assert_eq!(int("1_234_567_890"), Some(1234567890));
        assert_eq!(int("-12_34_56"), Some(-123456));
        assert_eq!(int("b1111_0000_1111"), Some(0xF0F));
        assert_eq!(int("x123_FED"), Some(0x123_FED));
        assert_eq!(int::<u8>("0.0"), None);
        assert_eq!(int::<u8>("255"), Some(255));
        assert_eq!(int::<u8>("256"), None);
        assert_eq!(int::<u8>("-1"), None);
        assert_eq!(int::<i8>("-128"), Some(-128));
        assert_eq!(int::<i8>("127"), Some(127));
        assert_eq!(int::<i8>("-129"), None);
        assert_eq!(int::<i8>("128"), None);
        assert_eq!(int::<i16>("+-0"), None);
        assert_eq!(int::<u32>("abc"), None);
        assert_eq!(int::<u32>("0o755"), None);
        assert_eq!(int::<i32>("0b0000_"), None);
        assert_eq!(int::<i32>("0b0000__0000"), None);
        assert_eq!(int::<i32>("0xBEEF"), None);
    }

    #[test]
    fn numbers() {
        assert_eq!(number::<f32>("+3.1415927").unwrap(), std::f32::consts::PI);
        assert_eq!(number::<f32>("-0.0").unwrap(), -0.0);
        assert_eq!(number::<f32>("1e15").unwrap(), 1e15);
        assert_eq!(number::<f32>("0.5431e-28").unwrap(), 0.5431e-28);
        assert_eq!(number::<f32>(".123456").unwrap(), 0.123456);
        assert_eq!(number::<f32>("0.1e1_2").unwrap(), 0.1e12);
        assert_eq!(number::<f32>("8_765.432").unwrap(), 8_765.432);
        assert_eq!(number::<f32>("100").unwrap(), 100.0);
        assert_eq!(number::<f32>("123_.456"), None);
        assert_eq!(number::<f32>("_123.456"), None);
        assert_eq!(number::<f32>("123.456_"), None);
        assert_eq!(number::<f32>("123._456"), None);
        assert_eq!(number::<f32>("12.34.56"), None);
        assert_eq!(number::<f64>("-123.456789e0").unwrap(), -123.456789);
        assert_eq!(number::<f64>("inf").unwrap(), std::f64::INFINITY);
        assert_eq!(number::<f64>("-inf").unwrap(), std::f64::NEG_INFINITY);
        assert_eq!(number::<f64>("1__0.0"), None);
        assert_eq!(number::<f64>("infinity"), None);
        assert_eq!(number::<f64>("INF"), None);
        assert_eq!(number::<f64>("nan"), None);
        assert_eq!(number::<f64>("nAn"), None);
        assert!(number::<f32>("NaN").unwrap().is_nan());
        assert!(number::<f32>("-NaN").unwrap().is_nan());
        assert!(number::<f32>("NaN").unwrap().is_sign_positive());
        assert!(number::<f32>("-NaN").unwrap().is_sign_negative());
    }
}
