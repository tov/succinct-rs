use std::cmp;
use std::env;
use std::fs;
use std::path::Path;

// We want to precompute a table for binomial coefficients ahead of
// time, since computing them on the fly is expensive.  First, we
// can build the table using the recurrence:
//
// B(n, n) = 1
// B(n, k) = B(n - 1, k - 1) + B(n - 1, k)
//
// Here's the first few rows, where n is the row number (starting at zero), and
// k is the column number (also starting at zero).
//
// 1
// 1 1
// 1 2 1
// 1 3 3 1
// 1 4 6 4 1
// ...
//
// We can concatenate the rows into a flat array.  Then, computing B(n, k)
// involves finding the start of the nth row, and then looking up the kth
// element in that row. The ith row has length i + 1, so then the nth row starts
// at 1 + 2 + ... + n, which is n * (n + 1) / 2.
//
// However, note that each row in the table above is symmetric:  B(n, k) =
// B(n, n - k).  So, we can cut our space usage in half by only storing the
// first half of each row.
//
// 1
// 1
// 1 2
// 1 3
// 1 4 6
// ...
//
// We need to be able to compute the length of a row and also efficiently compute
// the beginning of each row in our array, the sum of the previous rows' lengths.
// Previously, row i had length i + 1, and now it has length ceil((i + 1) / 2).
// We can then use the identity `ceil((n + 1) / m) = floor(n / m) + 1` to
// simplify this to `i // 2 + 1`, where `//` is integer division.
//
// Then, the start of row `n` is `\sum_{i=0}^{n-1} {i // 2 + 1}`, which we'd
// like to reduce to something closed-form.  Here's the first few values:
//
// n:         0 1 2 3 4 5 ...
// row_len:   1 1 2 2 3 3 ...
// row_start: 0 1 2 4 6 9 ...
//
// Let's assume `n` is even.  Then, note that summing the `row_len`s to the left
// is just `2 * (1 + 2 + ... + n/2)`.  Then, we have
//
// row_start(2m) = 2 * (1 + 2 + ... + m)
//               = 2 * m * (m + 1) / 2
//               = m * (m + 1)
//
// Then, if `n` is odd, we need to add `row_len(n - 1)`

// Note that if `n` is even, `2 * (1 + 2 + ... + n / 2)`, and if `n` is odd,
// we just need to add in its own row length to account for row `n - 1`.
//
// row_start(2m + 1) = m * (m + 1) + row_len(2m)
//                   = m * (m + 1) + (m + 1)
//                   = (m + 1) * (m + 1)
//
// Now, we can combine the two cases:
//
// row_start(n) = (n / 2 + n % 2) * (n / 2 + 1)
//
fn row_start(n: usize) -> usize {
    let (q, r) = (n / 2, n % 2);
    (q + r) * (q + 1)
}

fn row_len(n: usize) -> usize {
    n / 2 + 1
}

fn lookup(row: &[u64], n: usize, k: usize) -> u64 {
    row[cmp::min(k, n - k)]
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").expect("Failed to get output directory");
    let dst_path = Path::new(&out_dir).join("binomial.rs");

    let mut table = vec![];

    // Base case for n = 0;
    table.push(1u64);

    for n in 1..65usize {
        // Base case for k = 0
        table.push(1);
        for k in 1..row_len(n) {
            let prev_start = row_start(n - 1);
            let prev_row = &table[prev_start..(prev_start + row_len(n - 1))];
            let val = lookup(prev_row, n - 1, k - 1) + lookup(prev_row, n - 1, k);
            table.push(val);
        }
    }

    let code = format!("pub const COEFFICIENT_TABLE: &[u64; {}] = &{:?};", table.len(), table);

    fs::write(&dst_path, code).expect("Failed to write binomial coefficient table");
    println!("cargo:rerun-if-changed=build.rs");
}
