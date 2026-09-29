//! Ranking against ADR-0006. The oracle is an independent reference, Newton's
//! method on the log-likelihood in the logarithmic parameterization with the
//! host's `ln` and `exp`, a different route from the core's MM iteration;
//! and the relations the ADR states before the code.

use jacquard_core::judgment::Verdict::{self, First, Second, Tie};
use jacquard_core::rank::{Comparison, FLOOR, SplitMix64, rank, strengths};

fn c(first: usize, second: usize, verdict: Verdict) -> Comparison {
    Comparison {
        first,
        second,
        verdict,
    }
}

fn half_units(items: usize, comparisons: &[Comparison]) -> Vec<Vec<u32>> {
    let mut w = vec![vec![0u32; items]; items];
    for x in comparisons {
        match x.verdict {
            First => w[x.first][x.second] += 2,
            Second => w[x.second][x.first] += 2,
            Tie => {
                w[x.first][x.second] += 1;
                w[x.second][x.first] += 1;
            }
        }
    }
    w
}

fn sigma(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Newton's method on the penalized log-likelihood, theta = ln p, with the
/// phantom at theta = 0 playing two half-unit wins and two losses per item.
fn reference(wins: &[Vec<u32>]) -> Vec<f64> {
    let n = wins.len();
    let w = |i: usize, j: usize| f64::from(wins[i][j]);
    let mut theta = vec![0.0f64; n];
    for _ in 0..200 {
        let mut grad = vec![0.0; n];
        let mut hess = vec![vec![0.0; n]; n];
        for i in 0..n {
            let s0 = sigma(theta[i]);
            grad[i] += 2.0 - 4.0 * s0;
            hess[i][i] -= 4.0 * s0 * (1.0 - s0);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let games = w(i, j) + w(j, i);
                if games == 0.0 {
                    continue;
                }
                let s = sigma(theta[i] - theta[j]);
                grad[i] += w(i, j) - games * s;
                hess[i][i] -= games * s * (1.0 - s);
                hess[i][j] += games * s * (1.0 - s);
            }
        }
        // Solve hess * step = grad by Gaussian elimination with pivoting.
        let mut a: Vec<Vec<f64>> = hess
            .iter()
            .zip(&grad)
            .map(|(r, g)| r.iter().copied().chain([*g]).collect())
            .collect();
        for col in 0..n {
            let pivot = (col..n)
                .max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))
                .expect("rows");
            a.swap(col, pivot);
            for row in 0..n {
                if row != col {
                    let f = a[row][col] / a[col][col];
                    let pivot_row = a[col].clone();
                    for (x, q) in a[row][col..].iter_mut().zip(&pivot_row[col..]) {
                        *x -= f * q;
                    }
                }
            }
        }
        let mut largest = 0.0f64;
        for i in 0..n {
            let step = a[i][n] / a[i][i];
            theta[i] -= step;
            largest = largest.max(step.abs());
        }
        if largest < 1e-13 {
            break;
        }
    }
    theta.into_iter().map(f64::exp).collect()
}

fn random_comparisons(items: usize, count: usize, seed: u64) -> Vec<Comparison> {
    let mut rng = SplitMix64::new(seed);
    (0..count)
        .map(|_| {
            let a = rng.below(items);
            let b = (a + 1 + rng.below(items - 1)) % items;
            let verdict = match rng.below(5) {
                2 => Tie,
                k if (k < 2 && a < b) || k == 3 => First,
                _ => Second,
            };
            c(a, b, verdict)
        })
        .collect()
}

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    ((a - b) / b).abs() < tolerance
}

#[test]
fn the_mm_estimate_agrees_with_an_independent_newton_reference() {
    for (items, count, seed) in [(2, 5, 1), (3, 40, 2), (5, 120, 3), (8, 400, 4), (4, 3, 5)] {
        let comparisons = random_comparisons(items, count, seed);
        let wins = half_units(items, &comparisons);
        let mm = strengths(&wins);
        let newton = reference(&wins);
        for i in 0..items {
            assert!(
                close(mm[i], newton[i], 1e-8),
                "items {items} seed {seed} item {i}: mm {} newton {}",
                mm[i],
                newton[i]
            );
        }
    }
}

#[test]
fn a_clean_sweep_stays_finite() {
    let comparisons: Vec<_> = (0..30).map(|_| c(0, 1, First)).collect();
    let r = rank(2, &comparisons, 7);
    assert!(r[0].strength.is_finite() && r[0].strength > r[1].strength);
    assert!(r[1].strength > 0.0);
}

#[test]
fn record_order_does_not_change_the_ranking_bit_for_bit() {
    let comparisons = random_comparisons(6, 150, 11);
    let expected = rank(6, &comparisons, 42);
    let mut reversed = comparisons.clone();
    reversed.reverse();
    let mut rotated = comparisons.clone();
    rotated.rotate_left(37);
    for other in [reversed, rotated] {
        let got = rank(6, &other, 42);
        for (a, b) in expected.iter().zip(&got) {
            assert_eq!(a.strength.to_bits(), b.strength.to_bits());
            assert_eq!(a.low.to_bits(), b.low.to_bits());
            assert_eq!(a.high.to_bits(), b.high.to_bits());
            assert_eq!(a.rank, b.rank);
        }
    }
}

#[test]
fn relabeling_permutes_the_strengths() {
    let comparisons = random_comparisons(5, 100, 13);
    let perm = [3, 0, 4, 1, 2];
    let relabeled: Vec<_> = comparisons
        .iter()
        .map(|x| c(perm[x.first], perm[x.second], x.verdict))
        .collect();
    let a = rank(5, &comparisons, 1);
    let b = rank(5, &relabeled, 1);
    for i in 0..5 {
        assert!(close(a[i].strength, b[perm[i]].strength, 1e-9), "item {i}");
    }
}

#[test]
fn identical_records_give_equal_strengths() {
    // 0 and 1 split their games and have the same record against 2.
    let mut comparisons = vec![c(0, 1, First), c(1, 0, First), c(0, 1, Tie)];
    for _ in 0..4 {
        comparisons.extend([
            c(0, 2, First),
            c(2, 1, Second),
            c(2, 0, First),
            c(1, 2, Second),
        ]);
    }
    let r = rank(3, &comparisons, 3);
    assert!(
        close(r[0].strength, r[1].strength, 1e-12),
        "{} vs {}",
        r[0].strength,
        r[1].strength
    );
}

#[test]
fn a_win_never_lowers_the_winner_against_the_loser() {
    let comparisons = random_comparisons(4, 60, 17);
    let before = rank(4, &comparisons, 5);
    let mut more = comparisons.clone();
    more.push(c(2, 3, First));
    let after = rank(4, &more, 5);
    assert!(after[2].strength / after[3].strength >= before[2].strength / before[3].strength);
}

#[test]
fn ties_count_half_and_never_reverse_an_order() {
    let one_win = rank(2, &[c(0, 1, First)], 9);
    let mut with_ties = vec![c(0, 1, First)];
    with_ties.extend((0..20).map(|_| c(0, 1, Tie)));
    let tied = rank(2, &with_ties, 9);
    let ratio = |r: &[jacquard_core::rank::Estimate]| r[0].strength / r[1].strength;
    assert!(ratio(&tied) > 1.0, "a tie never reverses the order");
    assert!(
        ratio(&tied) < ratio(&one_win),
        "ties pull the pair together"
    );
    let all_ties = rank(2, &(0..10).map(|_| c(0, 1, Tie)).collect::<Vec<_>>(), 9);
    assert!(close(all_ties[0].strength, all_ties[1].strength, 1e-12));
}

#[test]
fn the_floor_leaves_thin_items_unranked() {
    let floor = usize::try_from(FLOOR).expect("small");
    // Item 0 plays 0 and 1 `floor` times; item 2 plays item 1 `floor - 1` times.
    let mut comparisons: Vec<_> = (0..floor)
        .map(|k| c(0, 1, if k % 3 == 0 { Second } else { First }))
        .collect();
    comparisons.extend((0..floor - 1).map(|_| c(1, 2, First)));
    let r = rank(3, &comparisons, 21);
    assert_eq!(r[0].comparisons, FLOOR);
    assert_eq!(r[2].comparisons, FLOOR - 1);
    assert!(r[0].rank.is_some() && r[1].rank.is_some());
    assert_eq!(r[2].rank, None, "below the floor");
    assert_eq!(r[0].rank, Some(1));
}

#[test]
fn intervals_bracket_the_estimate_and_depend_only_on_the_seed() {
    let comparisons = random_comparisons(4, 200, 23);
    let r = rank(4, &comparisons, 99);
    for e in &r {
        assert!(
            e.low <= e.strength * 1.000_001 && e.strength <= e.high * 1.000_001,
            "{e:?}"
        );
        assert!(e.low > 0.0 && e.high.is_finite());
    }
    let again = rank(4, &comparisons, 99);
    assert_eq!(r, again);
    let other_seed = rank(4, &comparisons, 100);
    assert!(
        r.iter()
            .zip(&other_seed)
            .any(|(a, b)| a.low.to_bits() != b.low.to_bits()),
        "the seed drives the bootstrap"
    );
}

#[test]
fn nothing_to_rank_is_all_phantom_and_unranked() {
    let r = rank(3, &[], 0);
    assert!(r.iter().all(|e| (e.strength - 1.0).abs() < f64::EPSILON
        && e.rank.is_none()
        && e.comparisons == 0));
    let self_play = rank(2, &[c(0, 0, First), c(0, 5, First)], 0);
    assert!(
        self_play.iter().all(|e| e.comparisons == 0),
        "self and out-of-range comparisons are ignored"
    );
}
