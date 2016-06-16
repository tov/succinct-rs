//! A trait for computing space usage.

use std::mem;

/// Types that know how to compute their space usage.
///
/// The space usage is split into two portions, the static portion
/// (returned by `static_bytes` and the dynamic portion (returned by
/// `dynamic_bytes`). The static portion is the statically-known size
/// for every object of its type; the dynamic portion is the additional
/// allocation that depends on run-time factors.
///
/// Examples:
///
///  - The size of primitive types like `u32` and `usize` is statically
///  known.
///
///  - The size of a tuple type is statically known when the sizes of
///  all its components are.
///
///  - The size of a vector includes a static portion, the vector struct
///  on the stack, and a dynamic portion, the heap array holding its
///  elements.
///
///  - The static size of a box includes its own stack space and the
///  static size of its contents (stored on the heap); the dynamic size
///  is the dynamic size of the contents.

pub trait SpaceUsage: Sized {
    /// Computes the size of the receiver in bytes.
    ///
    /// This includes not just the immediate stack object, but any heap
    /// memory that it owns.
    ///
    /// The default implementation returns
    /// `Self::static_bytes() + self.dynamic_bytes()`.
    #[inline]
    fn total_bytes(&self) -> usize {
        Self::static_bytes() + self.dynamic_bytes()
    }

    /// Is the size of this type known statically?
    ///
    /// If this method returns true then `dynamic_bytes` should always
    /// return 0.
    fn is_statically_sized() -> bool;

    /// Calculates the static portion of the size of this type.
    ///
    /// This is the minimum size that all objects of this type occupy,
    /// not counting storage that objects of the type might allocate
    /// dynamically.
    ///
    /// The default implementation returns `std::mem::size_of::<Self>()`.
    #[inline]
    fn static_bytes() -> usize {
        mem::size_of::<Self>()
    }

    /// Calculates the dynamic portion of the size of an object.
    ///
    /// This is the memory used by (or owned by) the object, not
    /// including any portion of its size that is known statically and
    /// included in `static_bytes`. This is typically for containers
    /// that heap allocate varying amounts of memory.
    ///
    /// The default implementation returns `0`.
    #[inline]
    fn dynamic_bytes(&self) -> usize {
        0
    }
}

#[macro_export]
macro_rules! impl_static_space_usage {
    ( $t:ty ) =>
    {
        impl SpaceUsage for $t {
            #[inline]
            fn is_statically_sized() -> bool { true }
        }
    }
}

impl_static_space_usage!(());
impl_static_space_usage!(u8);
impl_static_space_usage!(u16);
impl_static_space_usage!(u32);
impl_static_space_usage!(u64);
impl_static_space_usage!(usize);
impl_static_space_usage!(i8);
impl_static_space_usage!(i16);
impl_static_space_usage!(i32);
impl_static_space_usage!(i64);
impl_static_space_usage!(isize);
impl_static_space_usage!(f32);
impl_static_space_usage!(f64);

macro_rules! impl_tuple_space_usage {
    ( $( $tv:ident ),+ ) =>
    {
        impl<$( $tv: SpaceUsage ),+> SpaceUsage for ($( $tv, )+) {
            #[allow(non_snake_case)]
            fn dynamic_bytes(&self) -> usize {
                let &($( ref $tv, )+) = self;
                0 $( + $tv.dynamic_bytes() )+
            }

            #[inline]
            fn is_statically_sized() -> bool {
                return $( $tv::is_statically_sized() )&*;
            }
        }
    }
}

impl_tuple_space_usage!(A);
impl_tuple_space_usage!(A, B);
impl_tuple_space_usage!(A, B, C);
impl_tuple_space_usage!(A, B, C, D);
impl_tuple_space_usage!(A, B, C, D, E);
impl_tuple_space_usage!(A, B, C, D, E, F);
impl_tuple_space_usage!(A, B, C, D, E, F, G);
impl_tuple_space_usage!(A, B, C, D, E, F, G, H);
impl_tuple_space_usage!(A, B, C, D, E, F, G, H, I);
impl_tuple_space_usage!(A, B, C, D, E, F, G, H, I, J);
impl_tuple_space_usage!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple_space_usage!(A, B, C, D, E, F, G, H, I, J, K, L);

impl<A: SpaceUsage> SpaceUsage for Vec<A> {
    #[inline]
    fn is_statically_sized() -> bool {
        false
    }

    fn dynamic_bytes(&self) -> usize {
        if A::is_statically_sized() {
            self.capacity() * A::static_bytes()
        } else {
            let mut result = 0;
            for each in self {
                result += each.dynamic_bytes()
            }
            result
        }
    }
}

impl<A: SpaceUsage> SpaceUsage for Box<A> {
    #[inline]
    fn is_statically_sized() -> bool {
        A::is_statically_sized()
    }

    fn dynamic_bytes(&self) -> usize {
        use std::ops::Deref;
        self.deref().dynamic_bytes()
    }
}
