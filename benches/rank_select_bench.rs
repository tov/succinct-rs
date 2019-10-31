extern crate succinct;
extern crate criterion;
extern crate rand;

use succinct::bit_vec::{
    BitVector,
    BitVecPush,
};
use succinct::rsdic::RsDic;
use succinct::rank::{
    RankSupport,
    JacobsonRank,
    Rank9,
};
use succinct::select::{
    BinSearchSelect,
    Select0Support,
    Select1Support,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{SeedableRng, Rng};
use rand::rngs::StdRng;

const NUM_BITS: usize = 1_000_000;
const SEED: u64 = 88004802264174740;

fn random_bits(len: usize) -> BitVector<u64> {
    let mut rng = StdRng::seed_from_u64(SEED);
    let mut bv = BitVector::with_capacity(len as u64);
    for _ in 0..len {
        bv.push_bit(rng.gen());
    }
    bv
}

fn random_indices(count: usize, range: usize) -> Vec<usize> {
    let mut rng = StdRng::seed_from_u64(SEED);
    (0..count).map(|_| rng.gen_range(0, range)).collect()
}

fn bench_one_rank<R, F>(c: &mut Criterion, name: &str, f: F)
    where R: RankSupport<Over=bool>, F: FnOnce(BitVector<u64>) -> R
{
    let r = f(random_bits(NUM_BITS));
    let indices = random_indices(1, NUM_BITS);

    c.bench_function(name, |b| b.iter(|| {
        for &ix in &indices {
            r.rank(black_box(ix as u64), black_box(true));
        }
    }));
}

fn bench_rank(c: &mut Criterion) {
    bench_one_rank(c, "rsdic::rank", |bits| {
        let mut rs_dict = RsDic::with_capacity(NUM_BITS);
        for b in bits.iter() {
            rs_dict.push(b);
        }
        rs_dict
    });
    bench_one_rank(c, "jacobson::rank", JacobsonRank::new);
    bench_one_rank(c, "rank9::rank", Rank9::new);
}

fn bench_one_select<R, F>(c: &mut Criterion, name: &str, f: F)
    where R: Select0Support + Select1Support, F: Fn(BitVector<u64>) -> R
{
    let bits = random_bits(NUM_BITS);
    let num_set = bits.iter().filter(|&b| b).count();
    let r = f(bits);
    let indices = random_indices(1, num_set);

    c.bench_function(&format!("{}::select0", name), |b| b.iter(|| {
        for &ix in &indices {
            r.select0(black_box(ix as u64));
        }
    }));
    c.bench_function(&format!("{}::select1", name), |b| b.iter(|| {
        for &ix in &indices {
            r.select1(black_box(ix as u64));
        }
    }));
}

fn bench_select(c: &mut Criterion) {
    bench_one_select(c, "rsdic", |bits| {
        let mut rs_dict = RsDic::with_capacity(NUM_BITS);
        for b in bits.iter() {
            rs_dict.push(b);
        }
        rs_dict
    });
    bench_one_select(c, "rank9::binsearch", |b| BinSearchSelect::new(Rank9::new(b)));
}

criterion_group!(
    benches,
    bench_rank,
    bench_select
);
criterion_main!(benches);
