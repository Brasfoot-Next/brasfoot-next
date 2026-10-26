//! `Collections.sort` of the Java runtime (`java.util.TimSort`), for the game's comparators that
//! are not consistent (such as `h.a.abg`), where the result depends on the algorithm. For a
//! consistent comparator any stable sort gives the same order; use the standard one there.

use std::cmp::Ordering;

/// `TimSort.MIN_MERGE`: below it the whole list is one binary insertion sort.
const MIN_MERGE: usize = 32;

/// `Collections.sort(list, comparator)`.
pub fn sort_by<T: Copy>(list: &mut [T], mut compare: impl FnMut(&T, &T) -> Ordering) {
    let n = list.len();
    if n < 2 {
        return;
    }
    if n >= MIN_MERGE {
        panic!("not ported yet: TimSort merges (a list of {n} with an inconsistent comparator)");
    }
    let run = count_run_and_make_ascending(list, &mut compare);
    binary_sort(list, run, &mut compare);
}

/// `TimSort.countRunAndMakeAscending`
fn count_run_and_make_ascending<T: Copy>(a: &mut [T], compare: &mut impl FnMut(&T, &T) -> Ordering) -> usize {
    let hi = a.len();
    let mut run_hi = 1;
    if run_hi == hi {
        return 1;
    }
    run_hi += 1;
    if compare(&a[1], &a[0]) == Ordering::Less {
        while run_hi < hi && compare(&a[run_hi], &a[run_hi - 1]) == Ordering::Less {
            run_hi += 1;
        }
        a[..run_hi].reverse();
    } else {
        while run_hi < hi && compare(&a[run_hi], &a[run_hi - 1]) != Ordering::Less {
            run_hi += 1;
        }
    }
    run_hi
}

/// `TimSort.binarySort` from `start` (the first element after the leading run).
fn binary_sort<T: Copy>(a: &mut [T], start: usize, compare: &mut impl FnMut(&T, &T) -> Ordering) {
    let start = start.max(1);
    for i in start..a.len() {
        let pivot = a[i];
        let (mut left, mut right) = (0, i);
        while left < right {
            let mid = (left + right) >> 1;
            if compare(&pivot, &a[mid]) == Ordering::Less {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        a.copy_within(left..i, left + 1);
        a[left] = pivot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_stable_sort_for_a_consistent_comparator() {
        let mut a = [5, 1, 4, 1, 3, 9, 2, 6, 5, 3];
        let mut b = a;
        sort_by(&mut a, |x, y| x.cmp(y));
        b.sort();
        assert_eq!(a, b);
    }

    /// Inputs and outputs of `Collections.sort` on the JDK 21 for a comparator that says "before"
    /// for a larger first field and otherwise puts the larger second field first.
    #[test]
    fn matches_java_for_an_inconsistent_comparator() {
        let rule = |x: &(i32, i32), y: &(i32, i32)| if x.0 > y.0 { Ordering::Less } else { y.1.cmp(&x.1) };
        let cases: [(&[(i32, i32)], &[(i32, i32)]); 4] = [
            (&[(4,5),(4,0),(4,8),(4,0),(4,0),(2,8),(1,5),(2,1),(3,7),(4,2),(4,3),(1,8),(4,9),(0,6),(1,3),(3,9),(1,5),(2,8),(1,6)], &[(4,9),(3,9),(4,8),(1,8),(1,5),(4,3),(4,2),(3,7),(2,8),(2,8),(1,6),(1,3),(0,6),(4,5),(2,1),(1,5),(4,0),(4,0),(4,0)]),
            (&[(0,0),(3,7),(3,5),(2,9),(1,2),(2,5),(4,0),(4,2),(3,5),(1,7),(4,5),(1,9),(2,5),(0,5),(4,7),(4,2),(2,0)], &[(4,7),(4,2),(1,9),(4,5),(2,5),(4,2),(4,0),(2,9),(3,7),(1,7),(3,5),(3,5),(2,5),(2,0),(0,5),(1,2),(0,0)]),
            (&[(1,0),(1,1),(3,6),(4,7),(1,7),(1,8),(1,5),(3,6),(3,7),(2,6),(1,0),(4,5),(0,8),(1,1),(2,5),(2,5),(0,4)], &[(1,8),(0,8),(4,7),(4,5),(3,7),(1,7),(3,6),(3,6),(2,6),(2,5),(2,5),(1,5),(0,4),(1,1),(1,1),(1,0),(1,0)]),
            (&[(2,1),(2,4),(3,5),(2,2),(3,5),(1,9)], &[(1,9),(3,5),(3,5),(2,4),(2,2),(2,1)]),
        ];
        for (input, expected) in cases {
            let mut list = input.to_vec();
            sort_by(&mut list, rule);
            assert_eq!(list, expected);
        }
    }
}
