//! A trait for computing space usage.

use std::mem;

/// Types that know how to compute their space usage.
///
/// The space usage is split into two portions, the static portion
/// (returned by `stack_bytes` and the dynamic portion (returned by
/// `heap_bytes`). The static portion is the statically-known size
/// for every object of its type, allocated on the stack; the dynamic
/// portion is the additional heap allocation that may depend on run-time
/// factors.
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
///  elements, including both the static and dynamic sizes of the
///  elements.

pub trait SpaceUsage: Sized {
    /// Computes the size of the receiver in bytes.
    ///
    /// This includes not just the immediate stack object, but any heap
    /// memory that it owns.
    ///
    /// The default implementation returns
    /// `Self::stack_bytes() + self.heap_bytes()`.
    #[inline]
    fn total_bytes(&self) -> usize {
        Self::stack_bytes() + self.heap_bytes()
    }

    /// Is the size of this type known statically?
    ///
    /// If this method returns true then `heap_bytes` should always
    /// return 0.
    fn is_stack_only() -> bool;

    /// Calculates the static portion of the size of this type.
    ///
    /// This is the minimum size that all objects of this type occupy,
    /// not counting storage that objects of the type might allocate
    /// dynamically.
    ///
    /// The default implementation returns `std::mem::size_of::<Self>()`.
    #[inline]
    fn stack_bytes() -> usize {
        mem::size_of::<Self>()
    }

    /// Calculates the dynamic portion of the size of an object.
    ///
    /// This is the memory used by (or owned by) the object, not
    /// including any portion of its size that is known statically and
    /// included in `stack_bytes`. This is typically for containers
    /// that heap allocate varying amounts of memory.
    ///
    /// The default implementation returns `0`.
    #[inline]
    fn heap_bytes(&self) -> usize {
        0
    }
}

#[macro_export]
macro_rules! impl_static_space_usage {
    ( $t:ty ) =>
    {
        impl SpaceUsage for $t {
            #[inline]
            fn is_stack_only() -> bool { true }
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
            fn heap_bytes(&self) -> usize {
                let &($( ref $tv, )+) = self;
                0 $( + $tv.heap_bytes() )+
            }

            #[inline]
            fn is_stack_only() -> bool {
                return $( $tv::is_stack_only() )&*;
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

impl<A: SpaceUsage + ::std::fmt::Debug> SpaceUsage for Vec<A> {
    #[inline]
    fn is_stack_only() -> bool { false }

    fn heap_bytes(&self) -> usize {
        let mut result = self.capacity() * A::stack_bytes();

        if ! A::is_stack_only() {
            for each in self {
                result += each.heap_bytes();
            }
        }

        result
    }
}

impl<A: SpaceUsage> SpaceUsage for Box<A> {
    #[inline]
    fn is_stack_only() -> bool { false }

    fn stack_bytes() -> usize {
        mem::size_of::<Self>()
    }

    fn heap_bytes(&self) -> usize {
        use std::ops::Deref;
        self.deref().total_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn is_stack_only() {
        assert!(  u32::is_stack_only());
        assert!(  isize::is_stack_only());
        assert!(! Vec::<u64>::is_stack_only());
        assert!(! Vec::<Vec<u64>>::is_stack_only());
        assert!(  <(u32, u32, u32)>::is_stack_only());
        assert!(! <(u32, Vec<u32>, u32)>::is_stack_only());
    }

    #[test]
    fn int_size() {
        assert_eq!(2, 0u16.total_bytes());
        assert_eq!(4, 0u32.total_bytes());
        assert_eq!(8, 0i64.total_bytes());
    }

    #[test]
    fn tuple_size() {
        assert_eq!(8, (0u32, 0u32).total_bytes());
        // This isn’t guaranteed to work, but it does for now:
        assert_eq!(12, (0u32, 0u8, 0u32).total_bytes());
    }

    #[test]
    fn vec_size() {
        let v = Vec::<u64>::with_capacity(8);
        assert_eq!(8, v.capacity());
        assert_eq!(64, v.heap_bytes());
        assert_eq!(64 + size_of::<Vec<u64>>(),
                   v.total_bytes());
    }

    #[test]
    fn vec_vec_size() {
        let v1 = Vec::<u64>::with_capacity(8);
        let v2 = Vec::<u64>::with_capacity(8);
        let w = vec![v1, v2];
        assert_eq!(2, w.capacity());
        assert_eq!(128 + 2 * size_of::<Vec<u64>>() +
                      size_of::<Vec<Vec<u64>>>(),
                   w.total_bytes());
    }
}
