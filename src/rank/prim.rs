use rank::{BitRankSupport, RankSupport};
use storage::BlockType;

macro_rules! impl_rank_support_prim {
    ( $t:ident ) => {
        impl RankSupport for $t {
            type Over = bool;

            fn rank(&self, position: u64, value: bool) -> u64 {
                if value {
                    self.rank1(position)
                } else {
                    self.rank0(position)
                }
            }

            fn limit(&self) -> u64 {
                Self::nbits() as u64
            }
        }

        impl BitRankSupport for $t {
            fn rank1(&self, position: u64) -> u64 {
                debug_assert!(position < Self::nbits() as u64);

                let mask = Self::low_mask((position + 1) as usize);
                (*self & mask).count_ones() as u64
            }
        }
    };
}

impl_rank_support_prim!(u8);
impl_rank_support_prim!(u16);
impl_rank_support_prim!(u32);
impl_rank_support_prim!(u64);
impl_rank_support_prim!(usize);

#[cfg(test)]
mod test {
    use rank::*;

    #[test]
    fn rank1() {
        assert_eq!(0, 0b00000000u8.rank1(0));
        assert_eq!(0, 0b00000000u8.rank1(7));
        assert_eq!(1, 0b01010101u8.rank1(0));
        assert_eq!(1, 0b01010101u8.rank1(1));
        assert_eq!(2, 0b01010101u8.rank1(2));
        assert_eq!(2, 0b01010101u8.rank1(3));

        assert_eq!(3, 0b00001111u8.rank1(2));
        assert_eq!(4, 0b00001111u8.rank1(3));
        assert_eq!(4, 0b00001111u8.rank1(4));
        assert_eq!(4, 0b00001111u8.rank1(5));
        assert_eq!(4, 0b00001111u8.rank1(7));

        assert_eq!(0, 0b11110000u8.rank1(0));
        assert_eq!(0, 0b11110000u8.rank1(3));
        assert_eq!(1, 0b11110000u8.rank1(4));
        assert_eq!(2, 0b11110000u8.rank1(5));
        assert_eq!(4, 0b11110000u8.rank1(7));
    }

    #[test]
    fn rank0() {
        assert_eq!(1, 0b00000000u8.rank0(0));
        assert_eq!(8, 0b00000000u8.rank0(7));
        assert_eq!(0, 0b01010101u8.rank0(0));
        assert_eq!(1, 0b01010101u8.rank0(1));
        assert_eq!(1, 0b01010101u8.rank0(2));
        assert_eq!(2, 0b01010101u8.rank0(3));
    }

    #[test]
    fn rank() {
        assert_eq!(1, 0b00000000u8.rank(0, false));
        assert_eq!(8, 0b00000000u8.rank(7, false));
        assert_eq!(0, 0b01010101u8.rank(0, false));
        assert_eq!(1, 0b01010101u8.rank(1, false));
        assert_eq!(1, 0b01010101u8.rank(2, false));
        assert_eq!(2, 0b01010101u8.rank(3, false));

        assert_eq!(0, 0b00000000u8.rank(0, true));
        assert_eq!(0, 0b00000000u8.rank(7, true));
        assert_eq!(1, 0b01010101u8.rank(0, true));
        assert_eq!(1, 0b01010101u8.rank(1, true));
        assert_eq!(2, 0b01010101u8.rank(2, true));
        assert_eq!(2, 0b01010101u8.rank(3, true));
    }
}
