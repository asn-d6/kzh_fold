use ark_serialize::CanonicalSerialize;
use rand::Rng;
use std::fmt;
use std::ops::Add;
use ark_ff::{Field, Zero, PrimeField, FftField};
use itertools::Itertools;
use rand::RngCore;
use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
use crate::polynomial::lagrange_basis::{LagrangeBasis};
use crate::polynomial::univariate_poly::UnivariatePolynomial;
use crate::utils::{compute_powers, is_power_of_two};

/// We represent a bivariate polynomial in **Lagrange Basis Form**:
///
/// In the Lagrange basis, the polynomial is represented using the evaluation points and
/// corresponding Lagrange basis polynomials:
///
/// f(X, Y) = L_{0,0}(w_0, w_0)f(w_0, w_0) + L_{0,1}(w_0, w_1)f(w_0, w_1) + L_{0,2}(w_0, w_2)f(w_0, w_2) + f(w_0, w_3) +
///           L_{1,0}(w_1, w_0)f(w_1, w_0) + L_{1,1}(w_1, w_1)f(w_1, w_1) + L_{1,2}(w_1, w_2)f(w_1, w_2) + f(w_1, w_3) +
///           L_{2,0}(w_2, w_0)f(w_2, w_0) + L_{2,1}(w_2, w_1)f(w_2, w_1) + L_{2,2}(w_2, w_2)f(w_2, w_2) + f(w_2, w_3) +
///           L_{3,0}(w_3, w_0)f(w_3, w_0) + L_{3,1}(w_3, w_1)f(w_3, w_1) + L_{3,2}(w_3, w_2)f(w_3, w_2) + f(w_3, w_3)
///
/// Here, L_{i,j}(w_i, w_j) are the Lagrange basis polynomials evaluated at the points w_i and w_j, and f(w_i, w_j)
/// are the evaluations of the polynomial at those points. This form is particularly useful for polynomial interpolation.
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize)]
pub struct BivariatePolynomial<F: FftField> {
    // Flattened vector to represent the evaluations, where the entry at index (i, j) is located at i * degree_y + j
    pub evaluations: Vec<F>,
    // The lagrange basis used
    pub lagrange_basis_x: LagrangeBasis<F>,
    pub lagrange_basis_y: LagrangeBasis<F>,
    // Degree of the polynomial in both X and Y
    pub degree_x: usize,
    pub degree_y: usize,
}

impl<F: FftField + fmt::Display> fmt::Display for BivariatePolynomial<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "f(X, Y) =")?;
        for i in 0..self.degree_x {
            if i > 0 {
                write!(f, "+ ")?;
            }
            let mut first_term = true;
            for j in 0..self.degree_y {
                if !first_term {
                    write!(f, " + ")?;
                }
                write!(f, "f(w_{}, w_{})", i, j)?;
                write!(f, " ({})", self.evaluations[i * self.degree_y + j])?;
                first_term = false;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<F: FftField> Add for BivariatePolynomial<F> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        // Determine the maximum degree in x and y directions
        let new_degree_x = usize::max(self.degree_x, other.degree_x);
        let new_degree_y = usize::max(self.degree_y, other.degree_y);

        // Select the Lagrange basis for the larger polynomial in each direction
        let lagrange_basis_x = if self.degree_x >= other.degree_x {
            self.lagrange_basis_x
        } else {
            other.lagrange_basis_x
        };

        let lagrange_basis_y = if self.degree_y >= other.degree_y {
            self.lagrange_basis_y
        } else {
            other.lagrange_basis_y
        };

        // Initialize the resulting evaluations vector with zeros
        let mut evaluations = vec![F::zero(); new_degree_x * new_degree_y];

        // Perform element-wise addition in a single loop
        for i in 0..new_degree_x {
            for j in 0..new_degree_y {
                let idx_self = i * self.degree_y + j;
                let idx_other = i * other.degree_y + j;
                let idx_result = i * new_degree_y + j;

                if i < self.degree_x && j < self.degree_y {
                    evaluations[idx_result] += self.evaluations[idx_self];
                }
                if i < other.degree_x && j < other.degree_y {
                    evaluations[idx_result] += other.evaluations[idx_other];
                }
            }
        }

        BivariatePolynomial {
            evaluations,
            lagrange_basis_x,
            lagrange_basis_y,
            degree_x: new_degree_x,
            degree_y: new_degree_y,
        }
    }
}

impl<F: FftField> BivariatePolynomial<F> {
    pub fn new(
        evaluations: Vec<F>,
        domain_x: GeneralEvaluationDomain<F>,
        domain_y: GeneralEvaluationDomain<F>,
        degree_x: usize,
        degree_y: usize,
    ) -> Self {
        assert!(is_power_of_two(degree_x), "degree_x (upper bound) must be a power of two");
        assert!(is_power_of_two(degree_y), "degree_y (upper bound) must be a power of two");
        assert_eq!(evaluations.len(), degree_x * degree_y, "Evaluations length does not match the expected size");

        Self {
            evaluations,
            lagrange_basis_x: LagrangeBasis { domain: domain_x },
            lagrange_basis_y: LagrangeBasis { domain: domain_y },
            degree_x,
            degree_y,
        }
    }

    /// Generates a random BivariatePolynomial
    pub fn random<T: RngCore>(
        rng: &mut T,
        domain_x: GeneralEvaluationDomain<F>,
        domain_y: GeneralEvaluationDomain<F>,
        degree_x: usize,
        degree_y: usize,
    ) -> Self {
        assert!(is_power_of_two(degree_x), "degree_x (upper bound) must be a power of two");
        assert!(is_power_of_two(degree_y), "degree_y (upper bound) must be a power of two");

        let evaluations = (0..degree_x * degree_y).map(|_| F::rand(rng)).collect();

        BivariatePolynomial {
            evaluations,
            lagrange_basis_x: LagrangeBasis { domain: domain_x },
            lagrange_basis_y: LagrangeBasis { domain: domain_y },
            degree_x,
            degree_y,
        }
    }

    /// Generates a random BivariatePolynomial with binary coefficients
    /// XXX: Remove domain_x and domain_y as inputs
    pub fn random_binary<T: RngCore>(
        rng: &mut T,
        domain_x: GeneralEvaluationDomain<F>,
        domain_y: GeneralEvaluationDomain<F>,
        degree_x: usize,
        degree_y: usize,
    ) -> Self {
        assert!(is_power_of_two(degree_x), "degree_x (upper bound) must be a power of two");
        assert!(is_power_of_two(degree_y), "degree_y (upper bound) must be a power of two");

        let evaluations = (0..degree_x * degree_y).map(|_| {
            let random_bit = rng.gen_bool(0.5); // Generates a random boolean with equal probability
            if random_bit { F::one() } else { F::zero() }
        }).collect();

        BivariatePolynomial {
            evaluations,
            lagrange_basis_x: LagrangeBasis { domain: domain_x },
            lagrange_basis_y: LagrangeBasis { domain: domain_y },
            degree_x,
            degree_y,
        }
    }

    /// evaluation requires O(n^2) additions
    pub fn evaluate(&self, x: &F, y: &F) -> F {
        let l_x = self.lagrange_basis_x.evaluate(x);
        let l_y = self.lagrange_basis_y.evaluate(y);
        // the final result
        let mut sum = F::ZERO;
        for i in 0..self.degree_x {
            for j in 0..self.degree_y {
                sum += l_x[i] * l_y[j] * self.evaluations[i * self.degree_y + j];
            }
        }
        sum
    }

    /// f(x, Y) = sum_{i} L_i(x) * sum_{j} (L_j(Y) * f(w_i, w_j)) ===>
    /// f(x, w_t) = sum_{i} L_i(x) * sum_{j} (L_j(w_t) * f(w_i, w_j))
    ///           = sum_{i} L_i(x) * f(w_i, w_t))
    pub fn partially_evaluate_at_x(&self, x: &F) -> UnivariatePolynomial<F> {
        let l_x = self.lagrange_basis_x.evaluate(x);
        let mut evaluations = vec![F::ZERO; self.degree_y];
        for t in 0..self.degree_y {
            for i in 0..self.degree_x {
                evaluations[t] += l_x[i] * self.evaluations[i * self.degree_y + t];
            }
        }
        UnivariatePolynomial { evaluations, lagrange_basis: self.lagrange_basis_y.clone() }
    }

    /// f(X, y) = sum_{j} L_j(y) * sum_{i} (L_i(X) * f(w_i, w_j)) ===>
    /// f(w_t, y) = sum_{j} L_j(y) * sum_{i} (L_i(w_t) * f(w_i, w_j))
    ///           = sum_{j} L_j(y) * f(w_t, w_j))
    pub fn partially_evaluate_at_y(&self, y: &F) -> UnivariatePolynomial<F> {
        let l_y = self.lagrange_basis_y.evaluate(y);
        let mut evaluations = vec![F::ZERO; self.degree_x];
        for t in 0..self.degree_x {
            for j in 0..self.degree_y {
                evaluations[t] += l_y[j] * self.evaluations[t * self.degree_y + j];
            }
        }
        UnivariatePolynomial { evaluations, lagrange_basis: self.lagrange_basis_x.clone() }
    }

    /// Compute r(x) = \sum_{j\inH_y} f(X, j)
    ///
    /// Evaluates the polynomial at all roots of unity in the domain and sums the results.
    pub fn sum_partial_evaluations_in_domain(&self) -> UnivariatePolynomial<F> {
        // XXX This can probably be sped up...
        let mut r_poly = UnivariatePolynomial::new(vec![F::zero(); self.degree_x], self.lagrange_basis_x.domain.clone());
        for j in self.lagrange_basis_y.domain.elements() {
            r_poly = r_poly + self.partially_evaluate_at_y(&j);
        }

        r_poly
    }

    /// Computes the bitfield union of two bivariate polynomials.
    ///
    /// The coefficients of the resulting polynomial are the bitwise OR of the coefficients
    /// of the two input polynomials.
    pub fn bitfield_union(&self, other: &Self) -> Self {
        assert_eq!(self.degree_x, other.degree_x, "Polynomials must have the same degree in x direction");
        assert_eq!(self.degree_y, other.degree_y, "Polynomials must have the same degree in y direction");

        let evaluations: Vec<F> = self.evaluations.iter()
            .zip(&other.evaluations)
            .map(|(a, b)| {
                *a + *b - *a * *b // Since a, b are either 0 or 1, this is equivalent to a | b
            })
            .collect();

        Self {
            evaluations,
            lagrange_basis_x: self.lagrange_basis_x.clone(),
            lagrange_basis_y: self.lagrange_basis_y.clone(),
            degree_x: self.degree_x,
            degree_y: self.degree_y,
        }
    }

}

#[cfg(test)]
mod tests {
    use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
    use ark_std::UniformRand;
    use rand::thread_rng;
    use crate::constant_for_curves::ScalarField;
    use super::*;
    use crate::polynomial::univariate_poly::UnivariatePolynomial;

    type F = ScalarField;

    #[test]
    fn test_random_bivariate() {
        let degree_x = 4usize;
        let degree_y = 16usize;
        let domain_x = GeneralEvaluationDomain::<F>::new(degree_x).unwrap();
        let domain_y = GeneralEvaluationDomain::<F>::new(degree_y).unwrap();
        let r: BivariatePolynomial<F> = BivariatePolynomial::random(&mut thread_rng(), domain_x, domain_y, degree_x, degree_y);
        println!("{}", r);
    }

    #[test]
    fn test_partial_evaluation_x() {
        let degree_x = 4usize;
        let degree_y = 16usize;
        let domain_x = GeneralEvaluationDomain::<F>::new(degree_x).unwrap();
        let domain_y = GeneralEvaluationDomain::<F>::new(degree_y).unwrap();
        let r: BivariatePolynomial<F> = BivariatePolynomial::random(&mut thread_rng(), domain_x, domain_y, degree_x, degree_y);
        let x = F::rand(&mut thread_rng());
        let y = F::rand(&mut thread_rng());
        let r_x = r.partially_evaluate_at_x(&x);
        let r_xy_indirect = r_x.evaluate(&y);
        let r_xy_direct = r.evaluate(&x, &y);
        assert_eq!(r_xy_direct, r_xy_indirect);
    }

    #[test]
    fn test_partial_evaluation_y() {
        let degree_x = 16usize;
        let degree_y = 4usize;
        let domain_x = GeneralEvaluationDomain::<F>::new(degree_x).unwrap();
        let domain_y = GeneralEvaluationDomain::<F>::new(degree_y).unwrap();
        println!("{} {}", domain_x.size(), domain_y.size());
        let r: BivariatePolynomial<F> = BivariatePolynomial::random(&mut thread_rng(), domain_x, domain_y, degree_x, degree_y);
        let x = F::rand(&mut thread_rng());
        let y = F::rand(&mut thread_rng());
        let r_y = r.partially_evaluate_at_y(&y);
        let r_xy_indirect = r_y.evaluate(&x);
        let r_xy_direct = r.evaluate(&x, &y);
        assert_eq!(r_xy_direct, r_xy_indirect);
    }

}
