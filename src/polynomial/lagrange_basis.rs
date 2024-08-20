use ark_ff::FftField;
use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
use ark_serialize::CanonicalSerialize;

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize)]
pub struct LagrangeBasis<F: FftField> {
    pub domain: GeneralEvaluationDomain<F>,
}

impl<F: FftField> LagrangeBasis<F> {
    // TODO: optimize
    pub fn evaluate(&self, z: &F) -> Vec<F> {
        let mut evaluation_points = vec![];
        let eval = self.domain.evaluate_vanishing_polynomial(z.clone());

        for w_i in self.domain.elements() {
            if z == &w_i {
                // If z is one of the roots of unity, L_i(z) = 1 if z = w_i, otherwise 0
                evaluation_points.push(F::one());
            } else {
                // L_i(z) = w_i * eval / (z - w_i)
                evaluation_points.push((self.domain.size_inv() * w_i * eval) / (z.clone() - w_i));
            }
        }
        evaluation_points
    }

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
    use ark_ff::Field;
    use ark_poly::{EvaluationDomain, GeneralEvaluationDomain};
    use crate::constant_for_curves::ScalarField;
    use crate::polynomial::lagrange_basis::{LagrangeBasis};

    type F = ScalarField;

    #[test]
    fn lagrange_test() {
        let lagrange_basis = LagrangeBasis::new(10);
        assert_eq!(lagrange_basis.evaluate(&F::from(2u8)).len(), 16);
    }
}


