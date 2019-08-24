//! A trait for computing space usage.

use std::mem;

/// Computes the space usage of an object.
///
/// We calculate the space usage as split into two portions, the heap
/// portion (returned by `heap_bytes` and the stack portion (returned by
/// `stack_bytes`). The stack portion is the statically-known size for
/// every object of its type as allocated on the stack; the dynamic
/// portion is the additional heap allocation that may depend on
/// run-time factors.
///
/// Examples:
///
///  - Primitive types like `u32` and `usize` are stack-only.
///
///  - A tuple or struct type is stack-only when all its components are.
///    Its heap portion is the sum of their heap portions, but its stack
///    portion may exceed the sum of their stack portions because of
///    alignment and padding.
///
///  - The size of a vector includes a stack portion, the vector struct
///    itself, and a heap portion, the array holding its elements. The
///    heap portion of a vector includes the stack portions of its
///    elements. (Should they be called something else for this reason? I
///    considered static/dynamic, but `Box` shows why that doesn’t express
///    exactly the right property.)

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

    /// Calculates the stack portion of the size of this type.
    ///
    /// This is the size of the immediate storage that all objects of
    /// this type occupy; it excludes storage that objects of the
    /// type might allocate dynamically.
    ///
    /// The default implementation returns `std::mem::size_of::<Self>()`.

    #[inline]
    fn stack_bytes() -> usize {
        mem::size_of::<Self>()
    }

    /// Calculates the heap portion of the size of an object.
    ///
    /// This is the memory used by (or, rather, owned by) the object, not
    /// including any portion of its size that is
    /// included in `stack_bytes`. This is typically for containers
    /// that heap allocate varying amounts of memory.
    #[inline]
    fn heap_bytes(&self) -> usize;
}

impl_stack_only_space_usage!(());
impl_stack_only_space_usage!(u8);
impl_stack_only_space_usage!(u16);
impl_stack_only_space_usage!(u32);
impl_stack_only_space_usage!(u64);
impl_stack_only_space_usage!(usize);
impl_stack_only_space_usage!(i8);
impl_stack_only_space_usage!(i16);
impl_stack_only_space_usage!(i32);
impl_stack_only_space_usage!(i64);
impl_stack_only_space_usage!(isize);
impl_stack_only_space_usage!(f32);
impl_stack_only_space_usage!(f64);

impl<'a, T> SpaceUsage for &'a T {
    fn is_stack_only() -> bool {
        true
    }
    fn heap_bytes(&self) -> usize {
        0
    }
}

impl<'a, T> SpaceUsage for &'a [T] {
    fn is_stack_only() -> bool {
        true
    }
    fn heap_bytes(&self) -> usize {
        0
    }
}

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
                $( $tv::is_stack_only() )&*
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
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        let mut result = self.capacity() * A::stack_bytes();

        if !A::is_stack_only() {
            for each in self {
                result += each.heap_bytes();
            }
        }

        result
    }
}

impl<A: SpaceUsage> SpaceUsage for Box<A> {
    #[inline]
    fn is_stack_only() -> bool {
        false
    }

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
        assert!(u32::is_stack_only());
        assert!(isize::is_stack_only());
        assert!(!Vec::<u64>::is_stack_only());
        assert!(!Vec::<Vec<u64>>::is_stack_only());
        assert!(<(u32, u32, u32)>::is_stack_only());
        assert!(!<(u32, Vec<u32>, u32)>::is_stack_only());
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
        assert_eq!(64 + size_of::<Vec<u64>>(), v.total_bytes());
    }

    #[test]
    fn vec_vec_size() {
        let v1 = Vec::<u64>::with_capacity(8);
        let v2 = Vec::<u64>::with_capacity(8);
        let w = vec![v1, v2];
        assert_eq!(2, w.capacity());
        assert_eq!(
            128 + 2 * size_of::<Vec<u64>>() + size_of::<Vec<Vec<u64>>>(),
            w.total_bytes()
        );
    }
}
