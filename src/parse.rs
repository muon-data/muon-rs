// parse.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
use std::borrow::Cow;
use std::ops::Neg;
use std::str::FromStr;

const SIGNS: &[char] = &['-', '+'];

/// Integer trait
pub(crate) trait Integer: FromStr {
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
pub(crate) trait Number: FromStr + Neg<Output = Self> {
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
pub(crate) enum Sign {
    Negative,
    Positive,
}

/// Parse an integer from a string slice
pub(crate) fn int<T: Integer>(v: &str) -> Option<T> {
    if let Some(binary) = v.strip_prefix('b') {
        int_radix(binary, 2)
    } else if let Some(hexadecimal) = v.strip_prefix('x') {
        int_radix(hexadecimal, 16)
    } else {
        sanitize_num(v, 10)?.parse().ok()
    }
}

/// Parse an integer in an alternative radix
fn int_radix<T: Integer>(v: &str, radix: u32) -> Option<T> {
    // Do not allow signs in alternative radices
    (!v.starts_with(SIGNS)).then_some(())?;
    T::from_str_radix(&sanitize_num(v, radix)?, radix)
}

/// Parse a number from a string slice
pub(crate) fn number<T: Number>(v: &str) -> Option<T> {
    let (v, sign) = extract_sign(v);
    let first = match (v, sign) {
        ("inf", Sign::Negative) => return Some(T::NEG_INFINITY),
        ("inf", Sign::Positive) => return Some(T::INFINITY),
        ("NaN", sign) => return Some(T::nan(sign)),
        _ => v.chars().next()?,
    };
    // Check validity, sanitize, parse, and reïntroduce the sign
    (first.is_ascii_digit() || first == '.')
        .then(|| sanitize_num(v, 10)?.parse().ok())?
        .map(|v: T| if sign == Sign::Negative { -v } else { v })
}

/// Return the number literal with the sign separated out
fn extract_sign(v: &str) -> (&str, Sign) {
    if let Some(v) = v.strip_prefix('-') {
        (v, Sign::Negative)
    } else {
        (v.strip_prefix('+').unwrap_or(v), Sign::Positive)
    }
}

/// Sanitize a number, removing underscores, returning None if invalid placement
fn sanitize_num(value: &str, radix: u32) -> Option<Cow<'_, str>> {
    // If no underscores, return as-is, avoiding allocations
    if !value.contains('_') {
        return Some(value.into());
    }
    // Without the sign, if it exists, check for valid underscore placement
    for section in value.strip_prefix(SIGNS).unwrap_or(value).split('_') {
        // Characters surrounding `_` must be a digit in specified radix
        (section.chars().next()?.is_digit(radix)
            && section.chars().last()?.is_digit(radix))
        .then_some(())?;
    }
    // Strip out underscores
    Some(value.replace('_', "").into())
}

/// Parse a bool from a string slice
pub(crate) fn bool(value: &str) -> Option<bool> {
    value.parse().ok()
}

/// Parse a char (`text <=1 >=1`) from a string slice
pub(crate) fn char(value: &str) -> Option<char> {
    value.parse().ok()
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
        assert_eq!(int::<i32>("x-1Ac"), None);
        assert_eq!(int::<i32>("x+1Ac"), None);
        assert_eq!(int::<i32>("b-101010"), None);
        assert_eq!(int::<i32>("b+101010"), None);
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
        assert_eq!(number::<f32>("++0.123456"), None);
        assert_eq!(number::<f32>("+-0.123456"), None);
        assert_eq!(number::<f32>("-+0.123456"), None);
        assert_eq!(number::<f32>("--0.123456"), None);
        assert!(number::<f32>("NaN").unwrap().is_nan());
        assert!(number::<f32>("-NaN").unwrap().is_nan());
        assert!(number::<f32>("+NaN").unwrap().is_nan());
        assert!(number::<f32>("NaN").unwrap().is_sign_positive());
        assert!(number::<f32>("-NaN").unwrap().is_sign_negative());
        assert!(number::<f32>("+NaN").unwrap().is_sign_positive());
    }

    #[test]
    fn bools() {
        assert_eq!(bool("true"), Some(true));
        assert_eq!(bool("false"), Some(false));
        assert_eq!(bool("True"), None);
        assert_eq!(bool("False"), None);
        assert_eq!(bool("TRUE"), None);
        assert_eq!(bool("FALSE"), None);
    }

    #[test]
    fn chars() {
        assert_eq!(char(""), None);
        assert_eq!(char("aa"), None);
        assert_eq!(char("a"), Some('a'));
        assert_eq!(char("\0"), Some('\0'));
    }
}
