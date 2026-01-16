//! # Type Layouts

use core::alloc::Layout as BlockLayout;



#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub enum ReferenceLayout {
    Shared(DataLayout),
    Unique(DataLayout),
}

pub trait ReferenceType {
    const REFERENCE_LAYOUT: ReferenceLayout;
}

impl<T: ?Sized + DataType> ReferenceType for &T {
    const REFERENCE_LAYOUT: ReferenceLayout = ReferenceLayout::Shared(T::DATA_LAYOUT);
}

impl<T: ?Sized + DataType> ReferenceType for &mut T {
    const REFERENCE_LAYOUT: ReferenceLayout = ReferenceLayout::Unique(T::DATA_LAYOUT);
}

pub trait DataType {
    const DATA_LAYOUT: DataLayout;
}

pub trait SizedDataType: Sized {
    const SIZED_LAYOUT: SizedDataLayout;
}

impl<T: SizedDataType> DataType for T {
    const DATA_LAYOUT: DataLayout = T::SIZED_LAYOUT.to_unsized();
}

impl<T: SizedDataType> DataType for [T] {
    const DATA_LAYOUT: DataLayout = DataLayout::Slice(SliceLayout {
        element_layout: T::SIZED_LAYOUT,
    });
}

impl<T: SizedDataType, const N: usize> SizedDataType for [T; N] {
    const SIZED_LAYOUT: SizedDataLayout = SizedDataLayout::Array(ArrayLayout {
        length: N,
        element_layout: &T::SIZED_LAYOUT,
    });
}

macro_rules! impl_block_datatypes {
    ($($ty:ty)*) => {
        $(
            impl SizedDataType for $ty {
                const SIZED_LAYOUT: SizedDataLayout = SizedDataLayout::Block(
                    BlockLayout::new::<$ty>(),
                );
            }
        )*
    };
}

impl_block_datatypes! {
    u8 u16 u32 u64 u128
    i8 i16 i32 i64 i128
    f32 f64
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DataLayout {
    Array(ArrayLayout),
    Block(BlockLayout),
    Slice(SliceLayout),
    Struct(StructLayout),
    Unit,
}

impl DataLayout {
    pub const fn to_sized(self) -> Option<SizedDataLayout> {
        Some(match self {
            Self::Array(layout) => SizedDataLayout::Array(layout),
            Self::Block(layout) => SizedDataLayout::Block(layout),
            Self::Slice(_layout) => return None,
            Self::Struct(layout) => SizedDataLayout::Struct(layout),
            Self::Unit => SizedDataLayout::Unit,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub enum SizedDataLayout {
    Array(ArrayLayout),
    Block(BlockLayout),
    Struct(StructLayout),
    Unit,
}

impl SizedDataLayout {
    pub const fn to_unsized(self) -> DataLayout {
        match self {
            Self::Array(layout) => DataLayout::Array(layout),
            Self::Block(layout) => DataLayout::Block(layout),
            Self::Struct(layout) => DataLayout::Struct(layout),
            Self::Unit => DataLayout::Unit,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct ArrayLayout {
    pub length: usize,
    pub element_layout: &'static SizedDataLayout,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct SliceLayout {
    pub element_layout: SizedDataLayout,
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct StructLayout {
    pub layout: BlockLayout,
    pub fields: &'static [DataLayout],
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct FieldLayout {
    pub layout: BlockLayout,
    pub offset: usize,
}

// #[derive(Debug, Eq, PartialEq)]
// #[repr(C)]
// pub struct EnumLayout {
//     pub discriminant_layout: BlockLayout,
//     pub variants: &'static [DataLayout],
// }



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_layout_basics() {
        assert_eq!(
            <&[u8]>::REFERENCE_LAYOUT,
            ReferenceLayout::Shared(DataLayout::Slice(SliceLayout {
                element_layout: u8::SIZED_LAYOUT,
            })),
        );
        assert_eq!(
            <&mut [u8]>::REFERENCE_LAYOUT,
            ReferenceLayout::Unique(DataLayout::Slice(SliceLayout {
                element_layout: u8::SIZED_LAYOUT,
            })),
        );
    }

    #[test]
    fn block_data_layout_basics() {
        assert_eq!(u8::DATA_LAYOUT, i8::DATA_LAYOUT);
        assert_eq!(u16::DATA_LAYOUT, i16::DATA_LAYOUT);
        assert_eq!(u32::DATA_LAYOUT, i32::DATA_LAYOUT);
        assert_eq!(u64::DATA_LAYOUT, i64::DATA_LAYOUT);
        assert_eq!(u128::DATA_LAYOUT, i128::DATA_LAYOUT);

        assert_eq!(f32::DATA_LAYOUT, u32::DATA_LAYOUT);
        assert_eq!(f64::DATA_LAYOUT, u64::DATA_LAYOUT);

        assert_eq!(u8::SIZED_LAYOUT.to_unsized(), u8::DATA_LAYOUT);
    }
}
