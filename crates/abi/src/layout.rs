//! # Type Layouts

use core::alloc::Layout as BlockLayout;



#[derive(Debug, PartialEq)]
pub struct TypeDecl {
    pub name: &'static str,
    pub size: usize,
    pub align: usize,
    pub fields: &'static [FieldDecl],
}

impl TypeDecl {
    pub const UNIT: Self = Self {
        name: "()",
        size: size_of::<()>(),
        align: align_of::<()>(),
        fields: &[],
    };
}

#[derive(Debug, PartialEq)]
pub struct FieldDecl {
    pub offset: usize,
    pub decl: &'static TypeDecl,
}

#[derive(Debug, PartialEq)]
pub struct FunctionDecl {
    pub name: &'static str,
    pub input: &'static [&'static TypeDecl],
    pub output: &'static TypeDecl,
}

pub trait Declared {
    const DECL: &'static TypeDecl;

    fn alias_for<T: Declared>() -> bool {
        T::DECL.size == Self::DECL.size
            && T::DECL.align == Self::DECL.align
            && T::DECL.fields == Self::DECL.fields
    }
}

#[macro_export]
macro_rules! impl_declared {
    ($($ty:ty),* $(,)?) => {
        $(
            impl_declared!($ty {});
        )*
    };
    ($ty:ty { $($field_name:ident: $field_ty:ty),* $(,)? }) => {
        impl Declared for $ty {
            const DECL: &'static TypeDecl = &TypeDecl {
                name: stringify!($ty),
                size: size_of::<$ty>(),
                align: align_of::<$ty>(),
                fields: &[$(
                    FieldDecl {
                        offset: core::mem::offset_of!($ty, $field_name),
                        decl: <$field_ty as Declared>::DECL
                    }
                ),*],
            };
        }

        impl Declared for &$ty {
            const DECL: &'static TypeDecl = &TypeDecl {
                name: concat!("&", stringify!($ty)),
                size: size_of::<&$ty>(),
                align: align_of::<&$ty>(),
                fields: &[$(
                    FieldDecl {
                        offset: core::mem::offset_of!($ty, $field_name),
                        decl: <$field_ty as Declared>::DECL
                    }
                ),*],
            };
        }
    };
}

impl_declared!(u8, u16, u32, u64, u128);
impl_declared!(i8, i16, i32, i64, i128);

#[macro_export]
macro_rules! declare_function {
    (
        @ $decl_name: ident
        $vis:vis fn $name:ident($($param_name:ident: $param_ty:ty),* $(,)?) $(-> $return_ty:ty)? {
            $($body:tt)*
        }
    ) => {
        #[unsafe(no_mangle)]
        $vis extern "Rust" fn $name($($param_name: $param_ty),*) $(-> $return_ty)? {
            $($body)*
        }

        #[unsafe(no_mangle)]
        $vis static $decl_name: FunctionDecl = FunctionDecl {
            name: stringify!($name),
            input: &[$(
                <$param_ty as Declared>::DECL
            ),*],
            output: maybe_defined!($(<$return_ty as Declared>::DECL)? ; &TypeDecl::UNIT),
        };
    };
}

#[macro_export]
macro_rules! maybe_defined {
    ($expansion:expr ; $default:expr) => {
        $expansion
    };
    (; $default:expr) => {
        $default
    };
}



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

    #[test]
    fn type_declaration_basics() {
        struct TestType {
            a: u8,
            b: i32,
        }

        impl_declared! {
            TestType {
                a: u8,
                b: i32,
            }
        }

        assert_eq!(TestType::DECL.name, "TestType");
        assert_eq!(
            TestType::DECL.fields,
            &[
                FieldDecl {
                    offset: 4, // Note the compiler's reordering.
                    decl: u8::DECL,
                },
                FieldDecl {
                    offset: 0,
                    decl: i32::DECL,
                },
            ],
        );
    }

    #[test]
    fn function_declaration_basics() {
        declare_function! {
            @ ADD_ONE fn add_one(x: i32) -> i32 {
                x + 1
            }
        }
        assert_eq!(ADD_ONE.name, "add_one");
        assert_eq!(ADD_ONE.input, &[i32::DECL]);
        assert_eq!(ADD_ONE.output, i32::DECL);
    }
}
