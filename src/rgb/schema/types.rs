// LNP/BP Rust Library
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use num_traits::ToPrimitive;
use std::{convert::TryFrom, io};

pub trait UnsignedInteger:
    Clone + Copy + PartialEq + Eq + PartialOrd + Ord + Into<u64> + std::fmt::Debug
{
    const MAX: Self;

    fn as_u64(self) -> u64 {
        self.into()
    }

    fn bits() -> Bits;
}

impl UnsignedInteger for u8 {
    const MAX: Self = std::u8::MAX;

    #[inline]
    fn bits() -> Bits {
        Bits::Bit8
    }
}
impl UnsignedInteger for u16 {
    const MAX: Self = std::u16::MAX;

    #[inline]
    fn bits() -> Bits {
        Bits::Bit16
    }
}
impl UnsignedInteger for u32 {
    const MAX: Self = std::u32::MAX;

    #[inline]
    fn bits() -> Bits {
        Bits::Bit32
    }
}
impl UnsignedInteger for u64 {
    const MAX: Self = std::u64::MAX;

    #[inline]
    fn bits() -> Bits {
        Bits::Bit64
    }
}

pub trait Number:
    Clone + Copy + PartialEq + PartialOrd + std::fmt::Debug
{
}

impl Number for u8 {}
impl Number for u16 {}
impl Number for u32 {}
impl Number for u64 {}
impl Number for u128 {}
impl Number for usize {}
impl Number for i8 {}
impl Number for i16 {}
impl Number for i32 {}
impl Number for i64 {}
impl Number for i128 {}
impl Number for f32 {}
impl Number for f64 {}

/// NB: For now, we support only up to 128-bit integers and 64-bit floats;
/// nevertheless RGB schema standard allows up to 256-byte numeric types.
/// Support for larger types can be added later.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Display,
    ToPrimitive,
    FromPrimitive,
)]
#[display(Debug)]
#[repr(u8)]
#[non_exhaustive]
pub enum Bits {
    Bit8 = 1,
    Bit16 = 2,
    Bit32 = 4,
    Bit64 = 8,
    /* TODO: Add support later once bitcoin library will start supporting
     *       consensus-encoding of the native rust `u128` type
     *Bit128 = 16,
     *Bit256 = 32, */
}

impl Bits {
    pub fn max_value(&self) -> u128 {
        match *self {
            Bits::Bit8 => std::u8::MAX as u128,
            Bits::Bit16 => std::u16::MAX as u128,
            Bits::Bit32 => std::u32::MAX as u128,
            Bits::Bit64 => std::u64::MAX as u128,
            //Bits::Bit128 => std::u128::MAX as u128,
        }
    }

    pub fn byte_len(&self) -> usize {
        self.to_u8()
            .expect("Bit type MUST always occupy < 256 bytes") as usize
    }

    pub fn bit_len(&self) -> usize {
        self.byte_len() * 8
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
#[display(Debug)]
#[repr(u8)]
#[non_exhaustive]
pub enum Occurences<I>
where
    I: UnsignedInteger,
{
    Once,
    NoneOrOnce,
    OnceOrUpTo(Option<I>),
    NoneOrUpTo(Option<I>),
}

impl<I> Occurences<I>
where
    I: UnsignedInteger + From<u8>,
{
    pub fn min_value(&self) -> I {
        match self {
            Occurences::Once => I::from(1u8),
            Occurences::NoneOrOnce => I::from(0u8),
            Occurences::OnceOrUpTo(_) => I::from(1u8),
            Occurences::NoneOrUpTo(_) => I::from(0u8),
        }
    }

    pub fn max_value(&self) -> I {
        match self {
            Occurences::Once => I::from(1u8),
            Occurences::NoneOrOnce => I::from(1u8),
            Occurences::OnceOrUpTo(None) | Occurences::NoneOrUpTo(None) => {
                I::MAX
            }
            Occurences::OnceOrUpTo(Some(max))
            | Occurences::NoneOrUpTo(Some(max)) => *max,
        }
    }

    pub fn check<T>(&self, count: T) -> Result<(), OccurrencesError>
    where
        T: Number + Into<u128>,
        I: TryFrom<T> + Into<T> + Into<u128>,
        <I as TryFrom<T>>::Error: std::error::Error,
    {
        let orig_count = count;
        if count > I::MAX.into() {
            Err(OccurrencesError {
                min: self.min_value().into(),
                max: self.max_value().into(),
                found: count.into(),
            })?
        }
        let count = I::try_from(count).expect("Rust compiler is broken");
        match self {
            Occurences::Once if count == I::from(1u8) => Ok(()),
            Occurences::NoneOrOnce if count <= I::from(1u8) => Ok(()),
            Occurences::OnceOrUpTo(None) if count > I::from(0u8) => Ok(()),
            Occurences::OnceOrUpTo(Some(max))
                if count > I::from(0u8) && count <= *max =>
            {
                Ok(())
            }
            Occurences::NoneOrUpTo(None) => Ok(()),
            Occurences::NoneOrUpTo(Some(max)) if count <= *max => Ok(()),
            _ => Err(OccurrencesError {
                min: self.min_value().into(),
                max: self.max_value().into(),
                found: orig_count.into(),
            }),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
#[display(Debug)]
pub struct OccurrencesError {
    pub min: u128,
    pub max: u128,
    pub found: u128,
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Display,
    ToPrimitive,
    FromPrimitive,
)]
#[display(Debug)]
#[repr(u8)]
#[non_exhaustive]
pub enum DigestAlgorithm {
    // Single-path RIPEMD-160 is not secure and should not be used; see
    // <https://eprint.iacr.org/2004/199.pdf>
    //Ripemd160 = 0b_0000_1000_u8,
    Sha256 = 0b_0001_0001_u8,
    Sha512 = 0b_0001_0010_u8,
    Bitcoin160 = 0b_0100_1000_u8,
    Bitcoin256 = 0b_0101_0001_u8,
    /* Each tagged hash is a type on it's own, so the following umbrella
     * type was removed; a plain sha256 type must be used instead
     *Tagged256 = 0b_1100_0000_u8, */
}

pub mod elliptic_curve {
    use num_derive::{FromPrimitive, ToPrimitive};

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Debug,
        Display,
        ToPrimitive,
        FromPrimitive,
    )]
    #[display(Debug)]
    #[repr(u8)]
    #[non_exhaustive]
    pub enum EllipticCurve {
        Secp256k1 = 0x00,
        Curve25519 = 0x10,
    }

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Debug,
        Display,
        ToPrimitive,
        FromPrimitive,
    )]
    #[display(Debug)]
    #[repr(u8)]
    #[non_exhaustive]
    pub enum SignatureAlgorithm {
        Ecdsa = 0,
        Schnorr,
        Ed25519,
    }

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Debug,
        Display,
        ToPrimitive,
        FromPrimitive,
    )]
    #[display(Debug)]
    #[repr(u8)]
    #[non_exhaustive]
    pub enum PointSerialization {
        Uncompressed = 0,
        Compressed,
        SchnorrBip,
    }
}
pub use elliptic_curve::EllipticCurve;

mod strict_encoding {
    use super::*;
    use crate::strict_encoding::{Error, StrictDecode, StrictEncode};

    impl_enum_strict_encoding!(DigestAlgorithm);
    impl_enum_strict_encoding!(Bits);
    impl_enum_strict_encoding!(EllipticCurve);
    impl_enum_strict_encoding!(elliptic_curve::SignatureAlgorithm);
    impl_enum_strict_encoding!(elliptic_curve::PointSerialization);

    macro_rules! impl_occurences {
        ($type:ident) => {
            impl StrictEncode for Occurences<$type> {
                type Error = Error;

                fn strict_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Error> {
                    let value: (u8, u64) = match self {
                        Self::NoneOrOnce => (0x00u8, 0),
                        Self::Once => (0x01u8, 0),
                        Self::NoneOrUpTo(max) => (0xFEu8, max.unwrap_or(std::$type::MAX).into()),
                        Self::OnceOrUpTo(max) => (0xFFu8, max.unwrap_or(std::$type::MAX).into()),
                    };
                    let mut len = value.0.strict_encode(&mut e)?;
                    len += value.1.strict_encode(&mut e)?;
                    Ok(len)
                }
            }

            impl StrictDecode for Occurences<$type> {
                type Error = Error;

                #[allow(unused_comparisons)]
                fn strict_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
                    let value = u8::strict_decode(&mut d)?;
                    let max: u64 = u64::strict_decode(&mut d)?;
                    let max: Option<$type> = match max {
                        val if val >= 0 && val < ::std::$type::MAX.into() => {
                            Ok(Some($type::try_from(max).expect("Can't fail")))
                        }
                        val if val as u128 == ::std::$type::MAX as u128 => Ok(None),
                        invalid => Err(Error::ValueOutOfRange(
                            stringify!($type),
                            0..(::std::$type::MAX as u128),
                            invalid as u128,
                        )),
                    }?;
                    Ok(match value {
                        0x00u8 => Self::NoneOrOnce,
                        0x01u8 => Self::Once,
                        0xFEu8 => Self::NoneOrUpTo(max),
                        0xFFu8 => Self::OnceOrUpTo(max),
                        _ => panic!(
                            "New occurrence types can't appear w/o this library to be aware of"
                        ),
                    })
                }
            }
        };
    }

    impl_occurences!(u8);
    impl_occurences!(u16);
    impl_occurences!(u32);
    impl_occurences!(u64);
}

#[cfg(test)]
mod test {
    use super::Occurences;
    use super::*;
    use crate::strict_encoding::{test::*, StrictDecode};

    static ONCE: [u8; 9] = [0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];

    static NONEORONCE: [u8; 9] = [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];

    static NONEUPTO_U8: [u8; 9] =
        [0xfe, 0xff, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];

    static NONEUPTO_U16: [u8; 9] =
        [0xfe, 0xff, 0xff, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];

    static NONEUPTO_U32: [u8; 9] =
        [0xfe, 0xff, 0xff, 0xff, 0xff, 0x0, 0x0, 0x0, 0x0];

    static NONEUPTO_U64: [u8; 9] =
        [0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

    #[test]
    fn test_once_check_count() {
        let occurence: Occurences<u32> = Occurences::Once;
        occurence.check(1u32).unwrap();
    }
    #[test]
    #[should_panic(expected = "OccurrencesError { min: 1, max: 1, found: 0 }")]
    fn test_once_check_count_fail_zero() {
        let occurence: Occurences<u32> = Occurences::Once;
        occurence.check(0u32).unwrap();
    }
    #[test]
    #[should_panic(expected = "OccurrencesError { min: 1, max: 1, found: 2 }")]
    fn test_once_check_count_fail_two() {
        let occurence: Occurences<u32> = Occurences::Once;
        occurence.check(2u32).unwrap();
    }

    #[test]
    fn test_none_or_once_check_count() {
        let occurence: Occurences<u32> = Occurences::NoneOrOnce;
        occurence.check(1u32).unwrap();
    }
    #[test]
    fn test_none_or_once_check_count_zero() {
        let occurence: Occurences<u32> = Occurences::NoneOrOnce;
        occurence.check(0u32).unwrap();
    }
    #[test]
    #[should_panic(expected = "OccurrencesError { min: 0, max: 1, found: 2 }")]
    fn test_none_or_once_check_count_fail_two() {
        let occurence: Occurences<u32> = Occurences::NoneOrOnce;
        occurence.check(2u32).unwrap();
    }

    #[test]
    fn test_once_or_up_to_none() {
        let occurence: Occurences<u32> = Occurences::OnceOrUpTo(None);
        occurence.check(1u32).unwrap();
    }
    #[test]
    fn test_once_or_up_to_none_large() {
        let occurence: Occurences<u32> = Occurences::OnceOrUpTo(None);
        occurence.check(u32::MAX).unwrap();
    }
    #[test]
    #[should_panic(
        expected = "OccurrencesError { min: 1, max: 4294967295, found: 0 }"
    )]
    fn test_once_or_up_to_none_fail_zero() {
        let occurence: Occurences<u32> = Occurences::OnceOrUpTo(None);
        occurence.check(0u32).unwrap();
    }
    #[test]
    fn test_once_or_up_to_42() {
        let occurence: Occurences<u32> = Occurences::OnceOrUpTo(Some(42));
        occurence.check(42u32).unwrap();
    }
    #[test]
    #[should_panic(
        expected = "OccurrencesError { min: 1, max: 42, found: 43 }"
    )]
    fn test_once_or_up_to_42_large() {
        let occurence: Occurences<u32> = Occurences::OnceOrUpTo(Some(42));
        occurence.check(43u32).unwrap();
    }
    #[test]
    #[should_panic(expected = "OccurrencesError { min: 1, max: 42, found: 0 }")]
    fn test_once_or_up_to_42_fail_zero() {
        let occurence: Occurences<u32> = Occurences::OnceOrUpTo(Some(42));
        occurence.check(0u32).unwrap();
    }

    #[test]
    fn test_none_or_up_to_none_zero() {
        let occurence: Occurences<u32> = Occurences::NoneOrUpTo(None);
        occurence.check(0u32).unwrap();
    }
    #[test]
    fn test_none_or_up_to_none_large() {
        let occurence: Occurences<u32> = Occurences::NoneOrUpTo(None);
        occurence.check(u32::MAX).unwrap();
    }
    #[test]
    fn test_none_or_up_to_42_zero() {
        let occurence: Occurences<u32> = Occurences::NoneOrUpTo(Some(42));
        occurence.check(0u32).unwrap();
    }
    #[test]
    fn test_none_or_up_to_42() {
        let occurence: Occurences<u32> = Occurences::NoneOrUpTo(Some(42));
        occurence.check(42u32).unwrap();
    }
    #[test]
    #[should_panic(
        expected = "OccurrencesError { min: 0, max: 42, found: 43 }"
    )]
    fn test_none_or_up_to_42_large() {
        let occurence: Occurences<u32> = Occurences::NoneOrUpTo(Some(42));
        occurence.check(43u32).unwrap();
    }

    #[test]
    fn test_encode_occurance() {
        test_encode!(
            (ONCE, Occurences<u8>),
            (ONCE, Occurences<u16>),
            (ONCE, Occurences<u32>),
            (ONCE, Occurences<u64>),
            (NONEORONCE, Occurences<u8>),
            (NONEORONCE, Occurences<u16>),
            (NONEORONCE, Occurences<u32>),
            (NONEORONCE, Occurences<u64>)
        );

        test_encode!(
            (NONEUPTO_U8, Occurences<u8>),
            (NONEUPTO_U16, Occurences<u16>),
            (NONEUPTO_U32, Occurences<u32>),
            (NONEUPTO_U64, Occurences<u64>)
        );
    }

    #[test]
    fn test_encode_occurance_2() {
        let mut once_upto_u8 = NONEUPTO_U8.clone();
        let mut once_upto_u16 = NONEUPTO_U16.clone();
        let mut once_upto_u32 = NONEUPTO_U32.clone();
        let mut once_upto_u64 = NONEUPTO_U64.clone();

        once_upto_u8[0] = 0xFF;
        once_upto_u16[0] = 0xFF;
        once_upto_u32[0] = 0xFF;
        once_upto_u64[0] = 0xFF;

        let dec1: Occurences<u8> =
            Occurences::strict_decode(&once_upto_u8[..]).unwrap();
        let dec2: Occurences<u16> =
            Occurences::strict_decode(&once_upto_u16[..]).unwrap();
        let dec3: Occurences<u32> =
            Occurences::strict_decode(&once_upto_u32[..]).unwrap();
        let dec4: Occurences<u64> =
            Occurences::strict_decode(&once_upto_u64[..]).unwrap();

        assert_eq!(dec1, Occurences::OnceOrUpTo(None));
        assert_eq!(dec2, Occurences::OnceOrUpTo(None));
        assert_eq!(dec3, Occurences::OnceOrUpTo(None));
        assert_eq!(dec4, Occurences::OnceOrUpTo(None));

        let wc1: Occurences<u64> =
            Occurences::strict_decode(&once_upto_u8[..]).unwrap();
        let wc2: Occurences<u64> =
            Occurences::strict_decode(&once_upto_u16[..]).unwrap();
        let wc3: Occurences<u64> =
            Occurences::strict_decode(&once_upto_u32[..]).unwrap();

        assert_eq!(wc1, Occurences::OnceOrUpTo(Some(u8::MAX as u64)));
        assert_eq!(wc2, Occurences::OnceOrUpTo(Some(u16::MAX as u64)));
        assert_eq!(wc3, Occurences::OnceOrUpTo(Some(u32::MAX as u64)));
    }

    #[test]
    #[should_panic(
        expected = "New occurrence types can't appear w/o this library to be aware of"
    )]
    fn test_occurrence_panic_1() {
        test_garbage!((NONEUPTO_U8, Occurences<u8>));
    }

    #[test]
    #[should_panic(
        expected = "New occurrence types can't appear w/o this library to be aware of"
    )]
    fn test_occurrence_panic_2() {
        test_garbage!((NONEUPTO_U16, Occurences<u16>));
    }

    #[test]
    #[should_panic(
        expected = "New occurrence types can't appear w/o this library to be aware of"
    )]
    fn test_occurrence_panic_3() {
        test_garbage!((NONEUPTO_U32, Occurences<u32>));
    }

    #[test]
    #[should_panic(
        expected = "New occurrence types can't appear w/o this library to be aware of"
    )]
    fn test_occurrence_panic_4() {
        test_garbage!((NONEUPTO_U64, Occurences<u64>));
    }

    #[test]
    #[should_panic(expected = "ValueOutOfRange")]
    fn test_occurrence_panic_5() {
        test_encode!((NONEUPTO_U64, Occurences<u8>));
    }

    #[test]
    #[should_panic(expected = "ValueOutOfRange")]
    fn test_occurrence_panic_6() {
        test_encode!((NONEUPTO_U32, Occurences<u16>));
    }

    #[test]
    fn test_digest_algorithm() {
        let sha256 = DigestAlgorithm::Sha256;
        let sha512 = DigestAlgorithm::Sha512;
        let bitcoin160 = DigestAlgorithm::Bitcoin160;
        let bitcoin256 = DigestAlgorithm::Bitcoin256;

        print_bytes(&sha256);
        print_bytes(&sha512);
        print_bytes(&bitcoin160);
        print_bytes(&bitcoin256);

        let sha256_byte: [u8; 1] = [0x11];
        let sha512_byte: [u8; 1] = [0x12];
        let bitcoin160_byte: [u8; 1] = [0x48];
        let bitcoin256_byte: [u8; 1] = [0x51];

        test_encode!(
            (sha256_byte, DigestAlgorithm),
            (sha512_byte, DigestAlgorithm),
            (bitcoin160_byte, DigestAlgorithm),
            (bitcoin256_byte, DigestAlgorithm)
        );

        let sha256 = DigestAlgorithm::strict_decode(&[0x11][..]).unwrap();
        let sha512 = DigestAlgorithm::strict_decode(&[0x12][..]).unwrap();
        let bitcoin160 = DigestAlgorithm::strict_decode(&[0x48][..]).unwrap();
        let bitcoin256 = DigestAlgorithm::strict_decode(&[0x51][..]).unwrap();

        assert_eq!(sha256, DigestAlgorithm::Sha256);
        assert_eq!(sha512, DigestAlgorithm::Sha512);
        assert_eq!(bitcoin160, DigestAlgorithm::Bitcoin160);
        assert_eq!(bitcoin256, DigestAlgorithm::Bitcoin256);
    }

    #[test]
    #[should_panic(expected = "EnumValueNotKnown")]
    fn test_digest_panic() {
        DigestAlgorithm::strict_decode(&[0x17][..]).unwrap();
    }

    #[test]
    fn test_bits() {
        let bit8 = Bits::strict_decode(&[0x01][..]).unwrap();
        let bit16 = Bits::strict_decode(&[0x02][..]).unwrap();
        let bit32 = Bits::strict_decode(&[0x04][..]).unwrap();
        let bit64 = Bits::strict_decode(&[0x08][..]).unwrap();

        assert_eq!(bit8, Bits::Bit8);
        assert_eq!(bit16, Bits::Bit16);
        assert_eq!(bit32, Bits::Bit32);
        assert_eq!(bit64, Bits::Bit64);

        assert_eq!(bit8.max_value(), u8::MAX as u128);
        assert_eq!(bit16.max_value(), u16::MAX as u128);
        assert_eq!(bit32.max_value(), u32::MAX as u128);
        assert_eq!(bit64.max_value(), u64::MAX as u128);

        assert_eq!(bit8.bit_len(), 8 as usize);
        assert_eq!(bit8.byte_len(), 1 as usize);
        assert_eq!(bit16.bit_len(), 16 as usize);
        assert_eq!(bit16.byte_len(), 2 as usize);
        assert_eq!(bit32.bit_len(), 32 as usize);
        assert_eq!(bit32.byte_len(), 4 as usize);
        assert_eq!(bit64.bit_len(), 64 as usize);
        assert_eq!(bit64.byte_len(), 8 as usize);
    }

    #[test]
    #[should_panic(expected = "EnumValueNotKnown")]
    fn test_bits_panic() {
        Bits::strict_decode(&[0x12][..]).unwrap();
    }

    #[test]
    fn test_elliptic_curve() {
        let secp: [u8; 1] = [0x00];
        let c25519: [u8; 1] = [0x10];

        test_encode!(
            (secp, elliptic_curve::EllipticCurve),
            (c25519, elliptic_curve::EllipticCurve)
        );

        assert_eq!(
            elliptic_curve::EllipticCurve::strict_decode(&[0x00][..]).unwrap(),
            elliptic_curve::EllipticCurve::Secp256k1
        );

        assert_eq!(
            elliptic_curve::EllipticCurve::strict_decode(&[0x10][..]).unwrap(),
            elliptic_curve::EllipticCurve::Curve25519
        );
    }

    #[test]
    #[should_panic(expected = "EnumValueNotKnown")]
    fn test_elliptic_curve_panic() {
        elliptic_curve::EllipticCurve::strict_decode(&[0x09][..]).unwrap();
    }

    #[test]
    fn test_signature_algo() {
        let ecdsa_byte: [u8; 1] = [0x00];
        let schnorr_byte: [u8; 1] = [0x01];
        let ed25519_byte: [u8; 1] = [0x02];

        test_encode!(
            (ecdsa_byte, elliptic_curve::SignatureAlgorithm),
            (schnorr_byte, elliptic_curve::SignatureAlgorithm),
            (ed25519_byte, elliptic_curve::SignatureAlgorithm)
        );

        let ecdsa =
            elliptic_curve::SignatureAlgorithm::strict_decode(&[0x00][..])
                .unwrap();
        let schnorr =
            elliptic_curve::SignatureAlgorithm::strict_decode(&[0x01][..])
                .unwrap();
        let ed25519 =
            elliptic_curve::SignatureAlgorithm::strict_decode(&[0x02][..])
                .unwrap();

        assert_eq!(ecdsa, elliptic_curve::SignatureAlgorithm::Ecdsa);
        assert_eq!(schnorr, elliptic_curve::SignatureAlgorithm::Schnorr);
        assert_eq!(ed25519, elliptic_curve::SignatureAlgorithm::Ed25519);
    }

    #[test]
    #[should_panic(expected = "EnumValueNotKnown")]
    fn test_signature_algo_panic() {
        elliptic_curve::SignatureAlgorithm::strict_decode(&[0x03][..]).unwrap();
    }

    #[test]
    fn test_point_ser() {
        let uncompressed_byte: [u8; 1] = [0x00];
        let compressed_byte: [u8; 1] = [0x01];
        let schnorr_bip_byte: [u8; 1] = [0x02];

        test_encode!(
            (uncompressed_byte, elliptic_curve::PointSerialization),
            (compressed_byte, elliptic_curve::PointSerialization),
            (schnorr_bip_byte, elliptic_curve::PointSerialization)
        );

        assert_eq!(
            elliptic_curve::PointSerialization::strict_decode(&[0x00][..])
                .unwrap(),
            elliptic_curve::PointSerialization::Uncompressed
        );

        assert_eq!(
            elliptic_curve::PointSerialization::strict_decode(&[0x01][..])
                .unwrap(),
            elliptic_curve::PointSerialization::Compressed
        );

        assert_eq!(
            elliptic_curve::PointSerialization::strict_decode(&[0x02][..])
                .unwrap(),
            elliptic_curve::PointSerialization::SchnorrBip
        );
    }

    #[test]
    #[should_panic(expected = "EnumValueNotKnown")]
    fn test_point_ser_panic() {
        elliptic_curve::PointSerialization::strict_decode(&[0x03][..]).unwrap();
    }

    #[test]
    fn test_unsigned() {
        let u8_unsigned = u8::MAX;
        let u16_unsigned = u16::MAX;
        let u32_unsigned = u32::MAX;
        let u64_unsigned = u64::MAX;

        assert_eq!(u8_unsigned.as_u64(), u8::MAX as u64);
        assert_eq!(u8::bits(), Bits::Bit8);
        assert_eq!(u16_unsigned.as_u64(), u16::MAX as u64);
        assert_eq!(u16::bits(), Bits::Bit16);
        assert_eq!(u32_unsigned.as_u64(), u32::MAX as u64);
        assert_eq!(u32::bits(), Bits::Bit32);
        assert_eq!(u64_unsigned.as_u64(), u64::MAX as u64);
        assert_eq!(u64::bits(), Bits::Bit64);
    }
}
