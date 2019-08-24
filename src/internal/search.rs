use num_traits::PrimInt;

/// Averages two `PrimInt`s without overflowing.
pub fn average<P: PrimInt>(x: P, y: P) -> P {
    let almost_average = (x >> 1) + (y >> 1);
    let extra_bit = (x & P::one()) + (y & P::one()) >> 1;

    almost_average + extra_bit
}

/// Finds the smallest `d: D` in the interval `start .. limit` such
/// that `f(d) >= value`; requires that `f` be monotonically
/// non-decreasing.
///
/// Does not call `f` on `D`s outside the specified interval.
pub fn binary_search_function<D, R, F>(mut start: D, mut limit: D, value: R, f: F) -> Option<D>
where
    D: PrimInt,
    R: Ord,
    F: Fn(D) -> R,
{
    if start >= limit {
        return None;
    }
    if f(start) >= value {
        return Some(start);
    }

    // Now we know the answer isn't `start`, which means for every
    // candidate `mid`, `mid - 1` will still be in the domain of `f`.
    start = start + D::one();

    while start < limit {
        let mid = average(start, limit);

        if f(mid) >= value {
            if f(mid - D::one()) < value {
                return Some(mid);
            } else {
                limit = mid;
            }
        } else {
            start = mid + D::one();
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn avg_2_4() {
        assert_eq!(3, average(2, 4));
    }

    #[test]
    fn avg_2_5() {
        assert_eq!(3, average(2, 5));
    }

    #[test]
    fn avg_3_4() {
        assert_eq!(3, average(3, 4));
    }

    #[test]
    fn avg_3_5() {
        assert_eq!(4, average(3, 5));
    }

    #[test]
    fn avg_big() {
        let big: usize = !0;
        assert_eq!(big - 1, average(big, big - 1));
        assert_eq!(big - 1, average(big, big - 2));
        assert_eq!(big - 1, average(big - 1, big - 1));
        assert_eq!(big - 2, average(big - 2, big - 1));
        assert_eq!(big - 2, average(big - 2, big - 2));
        assert_eq!(big - 2, average(big - 1, big - 3));
    }

    fn search_slice(value: usize, slice: &[usize]) -> Option<usize> {
        binary_search_function(0, slice.len(), value, |index| slice[index])
    }

    const MAX_LEN: usize = 32;

    #[test]
    fn binary_search_01() {
        let mut vec = Vec::<usize>::with_capacity(MAX_LEN);

        for len in 0..MAX_LEN + 1 {
            for result in 0..len {
                vec.clear();
                for _ in 0..result {
                    vec.push(0);
                }
                for _ in result..len {
                    vec.push(1);
                }
                assert_eq!(Some(result), search_slice(1, &vec));
            }

            vec.clear();
            for _ in 0..len {
                vec.push(0)
            }
            assert_eq!(None, search_slice(1, &vec));
        }
    }

    #[test]
    fn binary_search_02() {
        let mut vec = Vec::<usize>::with_capacity(MAX_LEN);

        for len in 0..MAX_LEN + 1 {
            for result in 0..len {
                vec.clear();
                for _ in 0..result {
                    vec.push(0);
                }
                for _ in result..len {
                    vec.push(2);
                }
                assert_eq!(Some(result), search_slice(1, &vec));
            }

            vec.clear();
            for _ in 0..len {
                vec.push(0)
            }
            assert_eq!(None, search_slice(1, &vec));
        }
    }

    #[test]
    fn binary_search_iota() {
        let mut vec = Vec::<usize>::with_capacity(MAX_LEN);

        for len in 0..MAX_LEN + 1 {
            vec.clear();
            for i in 0..len {
                vec.push(i);
            }

            for i in 0..len {
                assert_eq!(Some(i), search_slice(i, &vec));
            }

            assert_eq!(None, search_slice(len, &vec));
        }
    }

    // #[test]
    // fn binary_search_2iota() {
    //     let mut vec = Vec::<usize>::with_capacity(MAX_LEN);

    //     for len in 0 .. MAX_LEN + 1 {
    //         vec.clear();
    //         for i in 0 .. len { vec.push(2 * i); }

    //         // assert_eq!(Some(0), search_slice(0, &vec));
    //         for i in 1 .. len {
    //             assert_eq!(Some(i), search_slice(2 * i, &vec));
    //             assert_eq!(Some(i), search_slice(2 * i - 1, &vec));
    //         }
    //     }
    // }
}
