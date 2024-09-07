use ark_ec::pairing::Pairing;
use ark_ff::PrimeField;

use crate::polynomial::multilinear_polynomial::decimal_to_boolean_vector;
use crate::polynomial::multilinear_polynomial::multilinear_poly::MultilinearPolynomial;
use crate::polynomial::multilinear_polynomial::math::Math;

pub struct BivariateMultiLinearPolynomial<F: PrimeField, E: Pairing> {
    pub poly: MultilinearPolynomial<F, E>,
    pub partial_multilinear: Vec<MultilinearPolynomial<F, E>>,
}

impl<F: PrimeField, E: Pairing<ScalarField=F>> BivariateMultiLinearPolynomial<F, E> {
    // this would output a bivariate multilinear polynomial consisting of n partial evaluations
    pub fn from_multilinear_to_bivariate_multilinear(poly: MultilinearPolynomial<F, E>, n: usize) -> BivariateMultiLinearPolynomial<F, E> {
        BivariateMultiLinearPolynomial {
            poly: poly.clone(),
            partial_multilinear: {
                let mut res = Vec::new();
                for i in 0..n {
                    let bits = decimal_to_boolean_vector(i, n.log_2());
                    res.push(poly.partial_evaluation(bits.as_slice()));
                }
                res
            },
        }
    }
}

impl<E: Pairing> BivariateMultiLinearPolynomial<E::ScalarField, E> {
    pub(crate) fn partial_evaluation(&self, input: &Vec<E::ScalarField>) -> MultilinearPolynomial<E::ScalarField, E> {
        self.poly.partial_evaluation(input)
    }

    pub(crate) fn partial_evaluations_over_boolean_domain(&self, i: usize) -> Vec<E::ScalarField> {
        self.partial_multilinear[i].evaluation_over_boolean_hypercube.clone()
    }
}

