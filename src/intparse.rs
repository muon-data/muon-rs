// intparse.rs
//
// Copyright (c) 2019  Douglas Lau
//
use num::{CheckedAdd, CheckedMul};
use num_traits::CheckedNeg;

/// Marker trait for integer types
pub trait Integer:
    Copy + PartialOrd + CheckedAdd + CheckedMul + CheckedNeg + From<u8>
{
}

impl Integer for u8 {}
impl Integer for i16 {}
impl Integer for u16 {}
impl Integer for i32 {}
impl Integer for u32 {}
impl Integer for i64 {}
impl Integer for u64 {}
impl Integer for i128 {}
impl Integer for u128 {}

/// Int parsing states
enum IntState<T: Integer> {
    Start,
    Positive,
    Negative,
    LeadingZero,
    Decimal(T),
    DecUnderscore(T),
    Binary(T),
    BinUnderscore(T),
    Octal(T),
    OctUnderscore(T),
    Hexadecimal(T),
    HexUnderscore(T),
    Invalid,
}

impl<T: Integer> IntState<T> {
    /// Convert a decimal char to T
    fn dec(c: char) -> T {
        T::from(c as u8 - b'0')
    }
    /// Append a char to a decimal value
    fn dec_append(v: T, c: char) -> Self {
        use IntState::*;
        if let Some(n) = v.checked_mul(&T::from(10)) {
            let d = Self::dec(c);
            if n >= T::from(0) {
                if let Some(n) = n.checked_add(&d) {
                    return Decimal(n);
                }
            } else if let Some(d) = d.checked_neg() {
                if let Some(n) = n.checked_add(&d) {
                    return Decimal(n);
                }
            }
        }
        Invalid
    }
    /// Append a char to a binary value
    fn bin_append(v: T, c: char) -> Self {
        use IntState::*;
        if let Some(n) = v.checked_mul(&T::from(2)) {
            if let Some(n) = n.checked_add(&Self::dec(c)) {
                return Binary(n);
            }
        }
        Invalid
    }
    /// Append a char to an octal value
    fn oct_append(v: T, c: char) -> Self {
        use IntState::*;
        if let Some(n) = v.checked_mul(&T::from(8)) {
            if let Some(n) = n.checked_add(&Self::dec(c)) {
                return Octal(n);
            }
        }
        Invalid
    }
    /// Convert a hexadecimal char to T
    fn hex(c: char) -> T {
        match c {
            '0'...'9' => T::from(c as u8 - b'0'),
            'A'...'F' => T::from(c as u8 - b'A' + 10),
            'a'...'f' => T::from(c as u8 - b'a' + 10),
            _ => panic!("invalid hex character: {}", c),
        }
    }
    /// Append a char to a hexadecimal value
    fn hex_append(v: T, c: char) -> Self {
        use IntState::*;
        if let Some(n) = v.checked_mul(&T::from(16)) {
            if let Some(n) = n.checked_add(&Self::hex(c)) {
                return Hexadecimal(n);
            }
        }
        Invalid
    }
    /// Parse next character
    fn parse_char(&self, c: char) -> Self {
        use IntState::*;
        match self {
            Start => match c {
                '+' => Positive,
                '-' => Negative,
                '0' => LeadingZero,
                '1'...'9' => Decimal(Self::dec(c)),
                _ => Invalid,
            },
            Positive => match c {
                '1'...'9' => Decimal(Self::dec(c)),
                _ => Invalid,
            },
            Negative => match c {
                '1'...'9' => {
                    if let Some(n) = Self::dec(c).checked_neg() {
                        return Decimal(n);
                    }
                    Invalid
                }
                _ => Invalid,
            },
            LeadingZero => match c {
                'b' => Binary(T::from(0)),
                'o' => Octal(T::from(0)),
                'x' => Hexadecimal(T::from(0)),
                _ => Invalid,
            },
            Decimal(v) => match c {
                '0'...'9' => Self::dec_append(*v, c),
                '_' => DecUnderscore(*v),
                _ => Invalid,
            },
            DecUnderscore(v) => match c {
                '0'...'9' => Self::dec_append(*v, c),
                _ => Invalid,
            },
            Binary(v) => match c {
                '0'...'1' => Self::bin_append(*v, c),
                '_' => BinUnderscore(*v),
                _ => Invalid,
            },
            BinUnderscore(v) => match c {
                '0'...'1' => Self::bin_append(*v, c),
                _ => Invalid,
            },
            Octal(v) => match c {
                '0'...'7' => Self::oct_append(*v, c),
                '_' => OctUnderscore(*v),
                _ => Invalid,
            },
            OctUnderscore(v) => match c {
                '0'...'7' => Self::oct_append(*v, c),
                _ => Invalid,
            },
            Hexadecimal(v) => match c {
                '0'...'9' | 'A'...'F' | 'a'...'f' => Self::hex_append(*v, c),
                '_' => HexUnderscore(*v),
                _ => Invalid,
            },
            HexUnderscore(v) => match c {
                '0'...'9' | 'A'...'F' | 'a'...'f' => Self::hex_append(*v, c),
                _ => Invalid,
            },
            _ => Invalid,
        }
    }

    /// Get the final integer value
    fn done(&self) -> Option<T> {
        use IntState::*;
        match self {
            LeadingZero => Some(T::from(0)),
            Decimal(v) => Some(*v),
            Binary(v) => Some(*v),
            Octal(v) => Some(*v),
            Hexadecimal(v) => Some(*v),
            _ => None,
        }
    }
}

/// Parse an integer from a string slice
pub fn from_str<T: Integer>(v: &str) -> Option<T> {
    let mut state = IntState::Start;
    for c in v.chars() {
        state = state.parse_char(c);
    }
    state.done()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ints() {
        assert_eq!(from_str("0"), Some(0));
        assert_eq!(from_str("25"), Some(25));
        assert_eq!(from_str("-42"), Some(-42));
        assert_eq!(from_str("+15"), Some(15));
        assert_eq!(from_str("0b101010"), Some(42u8));
        assert_eq!(from_str("0o755"), Some(493i16));
        assert_eq!(from_str("0x1Ac"), Some(428));
        assert_eq!(from_str("0xffff"), Some(0xFFFFu16));
        assert_eq!(from_str("0x1234567890"), Some(0x1234567890i64));
        assert_eq!(from_str("0x1000000000000000"), Some(0x1000000000000000u64));
        assert_eq!(
            from_str("0x10000000000000000"),
            Some(0x10000000000000000u128)
        );
        assert_eq!(from_str("1_234_567_890"), Some(1234567890));
        assert_eq!(from_str("0b_1111_0000_1111"), Some(0xF0F));
        assert_eq!(from_str("0o755_644"), Some(0o755644));
        assert_eq!(from_str::<u8>("0.0"), None);
        assert_eq!(from_str::<i16>("+-0"), None);
        assert_eq!(from_str::<u32>("00"), None);
        assert_eq!(from_str::<u32>("abc"), None);
        assert_eq!(from_str::<i32>("0b0000_"), None);
        assert_eq!(from_str::<i32>("0b0000__0000"), None);
    }
}
