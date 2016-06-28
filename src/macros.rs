//! Macros for export.

#[macro_export]
macro_rules! impl_bits_adapter {
    ( $block:ty, $field:ident )
        =>
    {
        type Block = $block;

        fn bit_len(&self) -> u64 { self.$field.bit_len() }

        fn block_len(&self) -> usize { self.$field.block_len() }

        fn get_block(&self, index: usize) -> $block {
            self.$field.get_block(index)
        }

        fn get_bit(&self, index: u64) -> bool {
            self.$field.get_bit(index)
        }

        fn get_bits(&self, index: u64, count: usize) -> $block {
            self.$field.get_bits(index, count)
        }
    }
}

#[macro_export]
macro_rules! impl_rank_support_adapter {
    ( $over:ty, $field:ident )
        =>
    {
        type Over = $over;

        fn rank(&self, index: u64, value: Self::Over) -> u64 {
            self.$field.rank(index, value)
        }

        fn limit(&self) -> u64 {
            self.$field.limit()
        }
    }
}

#[macro_export]
macro_rules! impl_bit_rank_support_adapter {
    ( $field:ident )
        =>
    {
        fn rank1(&self, index: u64) -> u64 {
            self.$field.rank1(index)
        }

        fn rank0(&self, index: u64) -> u64 {
            self.$field.rank0(index)
        }
    }
}

#[macro_export]
macro_rules! impl_select_support1_adapter {
    ( $field:ident )
        =>
    {
        fn select1(&self, index: u64) -> Option<u64> {
            self.$field.select1(index)
        }
    }
}

#[macro_export]
macro_rules! impl_select_support0_adapter {
    ( $field:ident )
        =>
    {
        fn select0(&self, index: u64) -> Option<u64> {
            self.$field.select0(index)
        }
    }
}

#[macro_export]
macro_rules! impl_select_support_adapter {
    ( $over:ty, $field:ident )
        =>
    {
        type Over = $over;

        fn select(&self, index: u64, value: Self::Over) -> Option<u64> {
            self.$field.select(index, value)
        }
    }
}
