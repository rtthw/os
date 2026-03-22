//! # Bit Manipulation Utilities

#![no_std]



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
}
