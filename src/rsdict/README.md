# RsDict: Fast rank/select over bitmaps
This data structure implements [Navarro and Providel, "Fast, Small, Simple
Rank/Select On Bitmaps"](https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf),
with heavy inspiration from a [Go implementation](https://github.com/hillbig/rsdic).

This data structure stores an append-only bitmap and provides two queries: rank and select.  First,
for some bitmap `B`, `rank(i)` counts the number of bits set to the left of `i`.  Then, `select(i)`
returns the index of the `i`th set bit, providing an inverse to `rank`.  These operations are useful
for building many different succinct data structures.  See Navarro's book on [Compact Data Structures](https://www.cambridge.org/core/books/compact-data-structures/68A5983E6F1176181291E235D0B7EB44) for an overview.

This library ports the Go implementation to Rust and adds a few optimizations.  First, the final phase
of computing a rank involves scanning over the compressed bitmap, decompressing it one block at a time
and keeping a running total of set bits.  For CPUs with SSSE3 support, this library performs this final
step without looping by using vectorized instructions.  Second, we use optimized routes for computing
`rank` and `select` within a single `u64`.  Rank uses `popcnt`, if available, and select implements
[this algorithm](https://lemire.me/blog/2018/02/21/iterating-over-set-bits-quickly/) to quickly skip over
unset bits.

## Performance
Here's some results from running the benchmark on my 2018 MacBook Pro with `-C target-cpu=native`.
```
rsdict::rank            time:   [10.330 us 10.488 us 10.678 us]
Found 4 outliers among 100 measurements (4.00%)
  4 (4.00%) high mild

jacobson::rank          time:   [17.958 us 18.335 us 18.740 us]
Found 6 outliers among 100 measurements (6.00%)
  1 (1.00%) high mild
  5 (5.00%) high severe

rank9::rank             time:   [6.8907 us 7.0768 us 7.2940 us]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high severe

rsdict::select0         time:   [37.124 us 37.505 us 37.991 us]
Found 3 outliers among 100 measurements (3.00%)
  3 (3.00%) high severe

rsdict::select1         time:   [29.782 us 29.918 us 30.067 us]
Found 7 outliers among 100 measurements (7.00%)
  5 (5.00%) high mild
  2 (2.00%) high severe

rank9::binsearch::select0
                        time:   [229.64 us 231.54 us 233.87 us]
Found 5 outliers among 100 measurements (5.00%)
  2 (2.00%) high mild
  3 (3.00%) high severe

rank9::binsearch::select1
                        time:   [253.69 us 255.84 us 258.19 us]
Found 9 outliers among 100 measurements (9.00%)
  4 (4.00%) high mild
  5 (5.00%) high severe
```
So for rank queries, this implementation is faster than `succinct-rs`'s Jacobson and slightly slower
than its Rank9.  However for select queries, it's *much* faster than doing binary search over these
rank structures, so consider using this library if you perform many selects.

## Testing
We generally use QuickCheck for testing data structure invariants.  In addition, there's basic AFL fuzz integration
to find interesting test cases using program coverage.  Install [cargo-afl](https://github.com/rust-fuzz/afl.rs)
and run the `rsdict_fuzz` binary with the `fuzz` feature set.
```
$ cargo install afl
$ cargo afl build --release --bin rsdict_fuzz --features fuzz

# Create some starting bitsets within target/fuzz/in and create an empty directory target/fuzz/out.
$ cargo afl fuzz -i target/fuzz/in -o target/fuzz/out target/release/rsdict_fuzz
```
