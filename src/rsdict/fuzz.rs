extern crate succinct;

use succinct::rank::RankSupport;
use succinct::rsdict::RsDict;
use succinct::select::SelectSupport;

fn main() {
    loop {
        afl::fuzz!(|data: &[u8]| {
            let mut bits = Vec::with_capacity(data.len() * 8);
            for byte in data {
                for i in 0..8 {
                    bits.push(byte & (1 << i) != 0);
                }
            }
            let mut blocks = Vec::with_capacity(bits.len() / 64);
            for chunk in bits.chunks_exact(64) {
                let mut block = 0;
                for (i, &bit) in chunk.iter().enumerate() {
                    if bit {
                        block |= 1 << i;
                    }
                }
                blocks.push(block);
            }

            let mut from_bits = RsDict::new();
            for &bit in &bits {
                from_bits.push(bit);
            }

            let mut from_blocks = RsDict::from_blocks(blocks.into_iter());
            for &bit in &bits[(bits.len() / 64 * 64)..] {
                from_blocks.push(bit);
            }

            let mut one_rank = 0;
            let mut zero_rank = 0;

            for (i, &bit) in bits.iter().enumerate() {
                for r in &[&from_bits, &from_blocks] {
                    assert_eq!(r.get_bit(i as u64), bit);

                    assert_eq!(r.rank(i as u64, false), zero_rank);
                    assert_eq!(r.rank(i as u64, true), one_rank);

                    if bit {
                        assert_eq!(r.select(one_rank as u64, true), Some(i as u64));
                    } else {
                        assert_eq!(r.select(zero_rank as u64, false), Some(i as u64));
                    }
                }
                if bit {
                    one_rank += 1;
                } else {
                    zero_rank += 1;
                }
            }

            for r in &[&from_bits, &from_blocks] {
                for rank in (one_rank + 1)..bits.len() as u64 {
                    assert_eq!(r.select(rank, true), None);
                }
                for rank in (zero_rank + 1)..bits.len() as u64 {
                    assert_eq!(r.select(rank, false), None);
                }
            }
        });
    }
}
