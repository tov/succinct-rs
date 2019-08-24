use super::{Select0Support, Select1Support, SelectSupport};
use bit_vec::BitVec;
use internal::search::binary_search_function;
use rank::{BitRankSupport, RankSupport};
use space_usage::SpaceUsage;

/// Performs a select query by binary searching rank queries.
pub struct BinSearchSelect<Rank> {
    rank_support: Rank,
}

/// Creates a new binary search select support based on a rank support.
impl<Rank: RankSupport> BinSearchSelect<Rank> {
    /// Creates a new binary search selection support given a rank
    /// support.
    pub fn new(rank_support: Rank) -> Self {
        BinSearchSelect {
            rank_support: rank_support,
        }
    }

    /// Borrows a reference to the underlying rank support.
    pub fn inner(&self) -> &Rank {
        &self.rank_support
    }

    /// Returns the underlying rank structure.
    pub fn into_inner(self) -> Rank {
        self.rank_support
    }
}

impl<Rank: BitVec> BitVec for BinSearchSelect<Rank> {
    impl_bit_vec_adapter!(Rank::Block, rank_support);
}

impl<Rank: RankSupport> RankSupport for BinSearchSelect<Rank> {
    impl_rank_support_adapter!(Rank::Over, rank_support);
}

impl<Rank: BitRankSupport> BitRankSupport for BinSearchSelect<Rank> {
    impl_bit_rank_support_adapter!(rank_support);
}

// If we had access to the representation of the rank structure, we
// could search level by level rather than at arbitrary bit addresses.
// But then this algorithm would be tied to that representation.

macro_rules! impl_select_support_b {
    ($select_support:ident, $select:ident, $rank: ident) => {
        impl<Rank: BitRankSupport> $select_support for BinSearchSelect<Rank> {
            fn $select(&self, index: u64) -> Option<u64> {
                binary_search_function(0, self.limit(), index + 1, |i| self.$rank(i))
            }
        }
    };
}

impl_select_support_b!(Select1Support, select1, rank1);
impl_select_support_b!(Select0Support, select0, rank0);

impl<Rank: RankSupport> SelectSupport for BinSearchSelect<Rank> {
    type Over = Rank::Over;

    fn select(&self, index: u64, value: Rank::Over) -> Option<u64> {
        binary_search_function(0, self.limit(), index + 1, |i| self.rank(i, value))
    }
}

impl<Rank: SpaceUsage> SpaceUsage for BinSearchSelect<Rank> {
    fn is_stack_only() -> bool {
        Rank::is_stack_only()
    }
    fn heap_bytes(&self) -> usize {
        self.rank_support.heap_bytes()
    }
}

#[cfg(test)]
mod test {
    use rank::*;
    use select::*;

    #[test]
    fn select1() {
        let vec = vec![0b00000000000001110000000000000001u32; 1024];
        let rank = JacobsonRank::new(vec);
        let select = BinSearchSelect::new(rank);

        assert_eq!(1, select.rank1(0));
        assert_eq!(1, select.rank1(1));
        assert_eq!(1, select.rank1(2));
        assert_eq!(1, select.rank1(15));
        assert_eq!(2, select.rank1(16));
        assert_eq!(3, select.rank1(17));
        assert_eq!(4, select.rank1(18));
        assert_eq!(4, select.rank1(19));
        assert_eq!(4, select.rank1(20));
        assert_eq!(5, select.rank1(32));

        assert_eq!(Some(0), select.select1(0));
        assert_eq!(Some(16), select.select1(1));
        assert_eq!(Some(17), select.select1(2));
        assert_eq!(Some(18), select.select1(3));
        assert_eq!(Some(32), select.select1(4));
        assert_eq!(Some(3200), select.select1(400));
        assert_eq!(Some(3216), select.select1(401));

        assert_eq!(Some(8 * 4092), select.select1(4092));
        assert_eq!(Some(8 * 4092 + 16), select.select1(4093));
        assert_eq!(Some(8 * 4092 + 17), select.select1(4094));
        assert_eq!(Some(8 * 4092 + 18), select.select1(4095));
        assert_eq!(None, select.select1(4096))
    }

    #[test]
    fn select2() {
        let vec = vec![0b10101010101010101010101010101010u32; 1024];
        let rank = JacobsonRank::new(vec);
        let select = BinSearchSelect::new(rank);

        assert_eq!(Some(1), select.select1(0));
        assert_eq!(Some(3), select.select1(1));
        assert_eq!(Some(5), select.select1(2));
        assert_eq!(Some(7), select.select1(3));
        assert_eq!(Some(919), select.select1(459));
    }

    #[test]
    fn select3() {
        let vec = vec![0b11111111111111111111111111111111u32; 1024];
        let rank = JacobsonRank::new(vec);
        let select = BinSearchSelect::new(rank);

        assert_eq!(Some(0), select.select1(0));
        assert_eq!(Some(1), select.select1(1));
        assert_eq!(Some(2), select.select1(2));
        assert_eq!(Some(32767), select.select1(32767));
        assert_eq!(None, select.select1(32768));
    }
}
