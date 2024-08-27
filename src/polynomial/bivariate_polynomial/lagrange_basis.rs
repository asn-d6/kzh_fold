use ark_ec::pairing::Pairing;
use ark_ff::{FftField, One, PrimeField};
use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
use ark_serialize::CanonicalSerialize;

use crate::polynomial::traits::Evaluable;

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize)]
pub struct LagrangeBasis<F: FftField> {
    pub domain: GeneralEvaluationDomain<F>,
}

impl<E: Pairing> Evaluable<E> for LagrangeBasis<E::ScalarField> {
    type Input = E::ScalarField;

    fn evaluate(&self, z: &Self::Input) -> Vec<E::ScalarField> {
        let mut evaluation_points = vec![];
        let eval = self.domain.evaluate_vanishing_polynomial(z.clone());

        for w_i in self.domain.elements() {
            if *z == w_i {
                // If z is one of the roots of unity, L_i(z) = 1 if z = w_i, otherwise 0
                evaluation_points.push(E::ScalarField::one());
            } else {
                // L_i(z) = w_i * eval / (z - w_i)
                evaluation_points.push((self.domain.size_inv() * w_i * eval) / (z.clone() - w_i));
            }
        }
        evaluation_points
    }
}


impl<F: FftField> LagrangeBasis<F> {
    pub fn evaluate_vanishing_polynomial(&self, z: &F) -> F {
        self.domain.evaluate_vanishing_polynomial(z.clone())
    }

    pub fn new(n: usize) -> Self {
        Self {
            domain: GeneralEvaluationDomain::<F>::new(n).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use ark_poly::EvaluationDomain;

    use crate::constant_for_curves::{E, ScalarField};
    use crate::polynomial::bivariate_polynomial::lagrange_basis::{Evaluable, LagrangeBasis};

    type F = ScalarField;

    #[test]
    fn lagrange_test() {
        let lagrange_basis: LagrangeBasis<F> = LagrangeBasis::new(10);
        assert_eq!(<LagrangeBasis<F> as Evaluable<E>>::evaluate(
            &lagrange_basis,
            &F::from(2u8)).len(),
            16,
        );
    }
}


