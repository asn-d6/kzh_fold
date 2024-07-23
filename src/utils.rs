use std::ops::Mul;
use ark_ff::{Field, One, PrimeField};
use rayon::prelude::*;

/// return [x^0, x^1, ..., x^n-1]
pub(crate) fn compute_powers<F: Field + Send + Sync>(x: &F, n: usize) -> Vec<F> {
    let sqrt_n = (n as f64).sqrt().ceil() as usize;

    let mut initial_powers = vec![F::ONE];
    let mut cur = *x;
    for _ in 1..sqrt_n {
        initial_powers.push(cur);
        cur *= x;
    }

    let powers: Vec<F> = (0..sqrt_n)
        .into_par_iter()
        .flat_map(|i| {
            let mut block = Vec::with_capacity(sqrt_n);
            if i == 0 {
                block.extend_from_slice(&initial_powers);
            } else {
                let initial_power = initial_powers[sqrt_n - 1] * x;
                let mut base = initial_powers[sqrt_n - 1] * x;
                for _ in 1..i {
                    base *= initial_power;
                }
                let mut cur = base;
                block.push(cur);
                for _ in 1..sqrt_n {
                    cur *= x;
                    block.push(cur);
                }
            }
            block
        })
        .collect();

    powers.into_iter().take(n).collect()
}


// It computes base^exponent efficiently with the minimal number of multiplications
pub(crate) fn power<F: Field>(base: F, exponent: usize) -> F {
    let binary_repr = format!("{:b}", exponent);
    let k = binary_repr.len();
    // Compute x^{2^t} for t < k
    let powers = {
        let mut powers = Vec::with_capacity(k);
        powers.push(base);
        for i in 1..k {
            let last_power = powers[i - 1];
            powers.push(last_power * last_power);
        }
        powers
    };
    // Compute the final result using the binary representation of exponent
    let mut result = F::ONE;
    for (i, bit) in binary_repr.chars().rev().enumerate() {
        if bit == '1' {
            result = powers[i] * result;
        }
    }
    result
}

pub(crate) fn is_power_of_two(n: usize) -> bool {
    // A number is a power of two if it is greater than 0 and
    // the bitwise AND of the number and one less than the number is 0.
    // This works because a power of two has exactly one bit set.
    n > 0 && (n & (n - 1)) == 0
}

pub(crate) fn inner_product<F: Field>(vector1: &[F], vector2: &[F]) -> F {
    // Check if the lengths of the vectors are the same
    assert_eq!(vector1.len(), vector2.len(), "The two vectors must have the same size.");
    // Compute the inner product
    vector1.iter().zip(vector2.iter()).map(|(a, b)| (*a) * (*b)).sum()
}


#[cfg(test)]
mod tests {
    use ark_bls12_381::Fr;
    use super::*;
    use ark_ff::{AdditiveGroup, Field, PrimeField};
    use ark_std::UniformRand;
    use rand::{Rng, thread_rng};

    pub(crate) fn compute_powers_non_parallel<F: Field>(x: &F, n: usize) -> Vec<F> {
        let mut powers = vec![F::ONE];
        let mut cur = *x;
        for _ in 0..n - 1 {
            powers.push(cur);
            cur *= x;
        }
        powers
    }

    type F = Fr;

    #[test]
    fn test_compute_powers() {
        let f = Fr::rand(&mut thread_rng());
        let n = 16;

        let result_original = compute_powers_non_parallel(&f, n);
        let result_parallel = compute_powers(&f, n);

        assert_eq!(result_original, result_parallel, "The results of the original and parallel implementations do not match.");

        let n = 32;

        let result_original = compute_powers_non_parallel(&f, n);
        let result_parallel = compute_powers(&f, n);

        assert_eq!(result_original, result_parallel, "The results of the original and parallel implementations do not match.");


        let n = 1234567;

        let result_original = compute_powers_non_parallel(&f, n);
        let result_parallel = compute_powers(&f, n);

        assert_eq!(result_original, result_parallel, "The results of the original and parallel implementations do not match.");


        let n = 1001;

        let result_original = compute_powers_non_parallel(&f, n);
        let result_parallel = compute_powers(&f, n);

        assert_eq!(result_original, result_parallel, "The results of the original and parallel implementations do not match.");
    }

    #[test]
    fn compute_pow_test() {
        assert_eq!(power(F::from(2u8), 0usize), F::ONE);
        assert_eq!(power(F::from(2u8), 1usize), F::from(2u8));
        assert_eq!(power(F::from(2u16), 9usize), F::from(512u16));
    }

    #[test]
    fn inner_product_test() {
        let vec1 = vec![F::ONE, F::ONE, F::ONE];
        let vec2 = vec![F::ZERO, F::ONE, F::ZERO];
        assert_eq!(inner_product(vec1.as_slice(), vec2.as_slice()), F::ONE);
        let vec1 = vec![F::ONE, F::ONE, F::ONE];
        let vec2 = vec![F::ONE, F::ONE, F::ZERO];
        assert_eq!(inner_product(vec1.as_slice(), vec2.as_slice()), F::from(2u8));
        let vec1 = vec![F::ONE, F::ONE, F::ONE];
        let vec2 = vec![F::ONE, F::ONE, F::ONE];
        assert_eq!(inner_product(vec1.as_slice(), vec2.as_slice()), F::from(3u8));
    }

    #[test]
    fn is_a_power_of_two_test() {
        assert_eq!(is_power_of_two(1), true);  // 2^0
        assert_eq!(is_power_of_two(2), true);  // 2^1
        assert_eq!(is_power_of_two(4), true);  // 2^2
        assert_eq!(is_power_of_two(8), true);  // 2^3
        assert_eq!(is_power_of_two(16), true); // 2^4
        assert_eq!(is_power_of_two(0), false); // 0 is not a power of two
        assert_eq!(is_power_of_two(3), false); // 3 is not a power of two
        assert_eq!(is_power_of_two(5), false); // 5 is not a power of two
        assert_eq!(is_power_of_two(6), false); // 6 is not a power of two
    }


}

