//! Macros for export.

/// Implements `SpaceUsage` for a stack-only (`Copy`) type.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate succinct;
/// use std::mem;
/// use succinct::SpaceUsage;
///
/// # #[allow(dead_code)]
/// struct Point { x: u32, y: u32 }
///
/// impl_stack_only_space_usage!(Point);
///
/// fn main() {
///     let point = Point { x: 0, y: 0 };
///     assert_eq!(point.total_bytes(), mem::size_of::<Point>());
/// }
/// ```
#[macro_export]
macro_rules! impl_stack_only_space_usage {
    ( $t:ty ) => {
        impl $crate::SpaceUsage for $t {
            #[inline]
            fn is_stack_only() -> bool {
                true
            }
            #[inline]
            fn heap_bytes(&self) -> usize {
                0
            }
        }
    };
}

/// Implements `BitVec` for a type that contains a `BitVec` field.
#[macro_export]
macro_rules! impl_bit_vec_adapter {
    ( $block:ty, $field:ident ) => {
        type Block = $block;

        fn bit_len(&self) -> u64 {
            self.$field.bit_len()
        }

        fn block_len(&self) -> usize {
            self.$field.block_len()
        }

        fn get_block(&self, index: usize) -> $block {
            self.$field.get_block(index)
        }

        fn get_bit(&self, index: u64) -> bool {
            self.$field.get_bit(index)
        }

        fn get_bits(&self, index: u64, count: usize) -> $block {
            self.$field.get_bits(index, count)
        }
    };
}

/// Implements `RankSupport` for a type that contains a `RankSupport` field.
#[macro_export]
macro_rules! impl_rank_support_adapter {
    ( $over:ty, $field:ident ) => {
        type Over = $over;

        fn rank(&self, index: u64, value: Self::Over) -> u64 {
            self.$field.rank(index, value)
        }

        fn limit(&self) -> u64 {
            self.$field.limit()
        }
    };
}

/// Implements `BitRankSupport` for a type that contains a `BitRankSupport`
/// field.
#[macro_export]
macro_rules! impl_bit_rank_support_adapter {
    ( $field:ident ) => {
        fn rank1(&self, index: u64) -> u64 {
            self.$field.rank1(index)
        }

        fn rank0(&self, index: u64) -> u64 {
            self.$field.rank0(index)
        }
    };
}

/// Implements `Select1Support` for a type that contains a `Select1Support`
/// field.
#[macro_export]
macro_rules! impl_select1_support_adapter {
    ( $field:ident ) => {
        fn select1(&self, index: u64) -> Option<u64> {
            self.$field.select1(index)
        }
    };
}

/// Implements `Select0Support` for a type that contains a `Select0Support`
/// field.
#[macro_export]
macro_rules! impl_select0_support_adapter {
    ( $field:ident ) => {
        fn select0(&self, index: u64) -> Option<u64> {
            self.$field.select0(index)
        }
    };
}

/// Implements `SelectSupport` for a type that contains a `SelectSupport`
/// field.
#[macro_export]
macro_rules! impl_select_support_adapter {
    ( $over:ty, $field:ident ) => {
        type Over = $over;

        fn select(&self, index: u64, value: Self::Over) -> Option<u64> {
            self.$field.select(index, value)
        }
    };
}
