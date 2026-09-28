//! Rectangular assignment (Kuhn–Munkres with potentials), O(n²·m).

/// Assign each of `n` rows to a distinct column (`n <= m`) maximising total
/// score. `score(r, c)` may return `f32::NEG_INFINITY` for forbidden pairs.
/// Returns the column for each row.
pub fn maximise(n: usize, m: usize, score: impl Fn(usize, usize) -> f32) -> Vec<usize> {
    assert!(n <= m, "more rows than columns");
    const BIG: f64 = 1e9;
    let cost = |r: usize, c: usize| -> f64 {
        let s = score(r, c);
        if s.is_finite() { -f64::from(s) } else { BIG }
    };
    // 1-indexed arrays per the classical formulation.
    let mut u = vec![0.0f64; n + 1];
    let mut v = vec![0.0f64; m + 1];
    let mut p = vec![0usize; m + 1];
    let mut way = vec![0usize; m + 1];
    let mut minv = vec![0.0f64; m + 1];
    let mut used = vec![false; m + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0usize;
        minv.iter_mut().for_each(|x| *x = f64::INFINITY);
        used.iter_mut().for_each(|x| *x = false);
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0usize;
            for j in 1..=m {
                if !used[j] {
                    let cur = cost(i0 - 1, j - 1) - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut out = vec![0usize; n];
    for j in 1..=m {
        if p[j] != 0 {
            out[p[j] - 1] = j - 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_optimal_assignment() {
        let s = [[9.0, 2.0, 7.0, 8.0], [6.0, 4.0, 3.0, 7.0], [5.0, 8.0, 1.0, 8.0]];
        let a = maximise(3, 4, |r, c| s[r][c]);
        let total: f32 = a.iter().enumerate().map(|(r, &c)| s[r][c]).sum();
        assert_eq!(total, 9.0 + 7.0 + 8.0);
        let mut cols = a.clone();
        cols.sort();
        cols.dedup();
        assert_eq!(cols.len(), 3);
    }

    #[test]
    fn respects_forbidden() {
        let a = maximise(2, 2, |r, c| if r == 0 && c == 0 { f32::NEG_INFINITY } else { 1.0 });
        assert_eq!(a, vec![1, 0]);
    }
}
