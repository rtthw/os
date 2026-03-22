//! # Bit Manipulation Utilities

#![no_std]



#[macro_export]
macro_rules! bit_flags {
    (
        $(#[$meta:meta])*
        $vis:vis struct $ident:ident: $ty:ty {
            $(
                $(#[$flag_meta:meta])*
                $flag_ident:ident @ $flag_bit:expr
            ),*
            $(,)?
        }
    ) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[allow(non_camel_case_types)]
        #[repr(transparent)]
        $(#[$meta])*
        $vis struct $ident($ty);

        #[allow(unused)]
        impl $ident {
            $(
                $(#[$flag_meta])*
                pub const $flag_ident: Self = Self(1 << $flag_bit);
            )*

            pub const NONE: Self = Self(0);
            pub const ALL: Self = Self(0 $(| 1 << $flag_bit)*);

            #[inline]
            pub const fn bits(&self) -> $ty {
                self.0
            }
        }

        impl ::core::fmt::Debug for $ident {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                if self == &Self::NONE {
                    return write!(f, concat!(stringify!($ident), "NONE"));
                }

                $(
                    if *self & Self::$flag_ident != Self::NONE {
                        write!(f, concat!(stringify!($flag_ident), "| "))?;
                    }
                )*

                Ok(())
            }
        }

        impl ::core::ops::BitOr for $ident {
            type Output = Self;

            fn bitor(self, rhs: Self) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl ::core::ops::BitAnd for $ident {
            type Output = Self;

            fn bitand(self, rhs: Self) -> Self::Output {
                Self(self.0 & rhs.0)
            }
        }
    };
}



#[macro_export]
macro_rules! bit_range {
    ($num:ident[$($start:ident)?..$($end:expr)?] $(as $ty:ty)?) => {{
        let width = $num.count_ones() + $num.count_zeros();
        let start = $crate::__expand_if_empty!($($start)? ; 0);

        $crate::bit_range!(
            @_done $num ; width ;
            start ;
            __expand_if_empty!($($end)? ; {
                start + __expand_if_empty!($(<$ty>::BITS)? ; width)
            })
        )
    }};
    ($num:ident[$($start:literal)?..$($end:expr)?] $(as $ty:ty)?) => {{
        let width = $num.count_ones() + $num.count_zeros();
        let start = $crate::__expand_if_empty!($($start)? ; 0);
        let end = $crate::__expand_if_empty!($($end)? ; {
            start + __expand_if_empty!($(<$ty>::BITS)? ; width)
        });

        $crate::bit_range!(@_done $num ; width ; start ; end )
    }};

    ($num:ident[$index:expr] $(as $ty:ty)?) => {{
        $num & (1 << $index) != 0
    }};

    (@_done $num:expr ; $width:expr ; $start:expr ; $end:expr) => {{
        if $start == $end {
            $num & (1 << $start)
        } else {
            let bits = $num << $width.saturating_sub($end) >> $width.saturating_sub($end);
            bits >> $start
        }
    }};
}

/// Internal utility macro that evaluates to the provided expansion if the input
/// before the semicolon is empty.
///
/// ## Examples
///
/// ```
/// use bit_utils::__expand_if_empty;
/// assert_eq!(__expand_if_empty!(0;1), 0);
/// assert_eq!(__expand_if_empty!( ;1), 1);
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! __expand_if_empty {
    ($something:expr ; $expansion:expr) => {
        $something
    };
    (; $expansion:expr) => {
        $expansion
    };
}



#[cfg(test)]
mod tests {
    #[test]
    #[rustfmt::skip]
    fn smoke() {
        let num: u64 = 43;
        assert_eq!(bit_range!(num[5..31] as u16), 1);

        let num: u8 = 0b_0011_0101;
        assert_eq!(bit_range!(num[0..0]), 0b_000001);
        assert_eq!(bit_range!(num[0..3]), 0b_000101);
        assert_eq!(bit_range!(num[2..6]), 0b_001101);
        assert_eq!(bit_range!(num[ .. ]), 0b_110101);

        let num: u32 = 0b_1001_0001;
        assert_eq!(bit_range!(num[4..4]), 0b_010000);
        assert_eq!(bit_range!(num[ ..3]), 0b_000001);
        assert_eq!(bit_range!(num[2.. ]), 0b_100100);
        assert_eq!(bit_range!(num[ ..5]), 0b_010001);

        // Make sure it works on constants.
        const NUM: u16 = 0b_1011_1111_0000_1101;
        assert_eq!(bit_range!(NUM[1..9 ] as u8), 0b_1000_0110);
        assert_eq!(bit_range!(NUM[8..  ] as u8), 0b_1011_1111);
        assert_eq!(bit_range!(NUM[4..12] as u8), 0b_1111_0000);

        const SOURCE_NUM: u32 = 0x_FEFE_FEFE;
        const RANGE_START: u32 = 2;
        const RANGE_END: u32 = 7;
        const RANGED_NUM: u32 = bit_range!(SOURCE_NUM[RANGE_START..RANGE_END] as u32);
        assert_eq!(RANGED_NUM, 31);
        assert_eq!(bit_range!(RANGED_NUM[0]), true);
        assert_eq!(bit_range!(RANGED_NUM[1]), true);
        assert_eq!(bit_range!(RANGED_NUM[2]), true);
        assert_eq!(bit_range!(RANGED_NUM[3]), true);
        assert_eq!(bit_range!(RANGED_NUM[4]), true);
        assert_eq!(bit_range!(RANGED_NUM[5]), false);
    }

    #[test]
    fn flags_smoke() {
        bit_flags! {
            /// Flag docs.
            struct Flags: u8 {
                /// Docs for A.
                A @ 0,
                /// Docs for B.
                B @ 2,
                /// Docs for C.
                C @ 7,
            }
        }

        assert_eq!(Flags::ALL, Flags::A | Flags::B | Flags::C);

        assert_eq!((Flags::A | Flags::B).bits(), 0b0000_0101);
        assert_eq!((Flags::B | Flags::C).bits(), 0b1000_0100);
        assert_eq!((Flags::A | Flags::C).bits(), 0b1000_0001);
    }
}
