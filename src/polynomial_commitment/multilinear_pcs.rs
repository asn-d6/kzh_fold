use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::Mul;

use ark_ec::{CurveGroup, VariableBaseMSM};
use ark_ec::pairing::Pairing;
use ark_std::UniformRand;
use rand::RngCore;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use crate::polynomial::bivariate_polynomial::univariate_poly::UnivariatePolynomial;
use crate::polynomial::multilinear_polynomial::dense_multilinear_poly::MultilinearPolynomial;
use crate::polynomial::multilinear_polynomial::math::Math;
use crate::polynomial::traits::{Evaluable, OneDimensionalPolynomial, TwoDimensionalPolynomial};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SRS<E: Pairing> {
    pub n: usize,
    pub m: usize,
    pub matrix_H: Vec<Vec<E::G1Affine>>,
    pub vec_H: Vec<E::G1Affine>,
    pub vec_V: Vec<E::G2>,
    pub V_prime: E::G2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commitment<E: Pairing> {
    pub C: E::G1Affine,
    pub aux: Vec<E::G1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningProof<E: Pairing, U: OneDimensionalPolynomial<E>> {
    pub vec_D: Vec<E::G1Affine>,
    pub f_star_poly: U,
}

// Define the new struct that encapsulates the functionality of polynomial commitment
pub struct PolyCommit<E: Pairing, U: OneDimensionalPolynomial<E>, B: TwoDimensionalPolynomial<E>> {
    pub srs: SRS<E>,
    pub(crate) phantom_data: PhantomData<fn() -> (U, B)>,
}

pub trait PolyCommitTrait<E: Pairing, U, B>
where
    U: OneDimensionalPolynomial<
        E,
        Input=Vec<<E as Pairing>::ScalarField>,
    >,
    B: TwoDimensionalPolynomial<
        E,
        Input=Vec<<E as Pairing>::ScalarField>,
        PartialEvalType=MultilinearPolynomial<E::ScalarField, E>
    >,
{
    fn setup<T: RngCore>(n: usize, m: usize, rng: &mut T) -> SRS<E>;

    fn commit(&self, poly: &B) -> Commitment<E>;

    fn open(&self,
            poly: &B,
            com: Commitment<E>,
            b: &U::Input,
    ) -> OpeningProof<E, U>;

    fn verify(&self,
              lagrange_x: &dyn Evaluable<E, Input=U::Input>,
              C: &Commitment<E>,
              proof: &OpeningProof<E, U>,
              b: &U::Input,
              c: &U::Input,
              y: &E::ScalarField,
    ) -> bool;
}

impl<E: Pairing, U, B> PolyCommitTrait<E, U, B> for PolyCommit<E, U, B>
where
    U: OneDimensionalPolynomial<
        E,
        Input=Vec<<E as Pairing>::ScalarField>,
    >,
    B: TwoDimensionalPolynomial<
        E,
        Input=Vec<<E as Pairing>::ScalarField>,
        PartialEvalType=MultilinearPolynomial<E::ScalarField, E>
    >,
{
    fn setup<T: RngCore>(n: usize, m: usize, rng: &mut T) -> SRS<E> {
        // sample G_0, G_1, ..., G_m generators from group one
        let G1_generator_vec = {
            let mut elements = Vec::new();
            for _ in 0..m {
                elements.push(E::G1Affine::rand(rng));
            }
            elements
        };
        // sample V, generator for group two
        let G2_generator = E::G2Affine::rand(rng);
        // sample trapdoors tau_0, tau_1, ..., tau_n, alpha
        let tau = {
            let mut elements = Vec::new();
            for _ in 0..n {
                elements.push(E::ScalarField::rand(rng));
            }
            elements
        };
        let alpha = E::ScalarField::rand(rng);
        // generate matrix_H
        let matrix_H: Vec<Vec<_>> = (0..n).into_par_iter()
            .map(|i| {
                let mut row = Vec::new();
                for j in 0..m {
                    let g = G1_generator_vec[j].mul(tau[i]);
                    row.push(g.into());
                }
                row
            })
            .collect();
        // generate vec_H
        let vec_H = {
            let mut vec_h = Vec::new();
            for j in 0..m {
                vec_h.push(G1_generator_vec[j].mul(alpha).into());
            }
            vec_h
        };
        // generate vec_V
        let vec_V = {
            let mut vec_h = Vec::new();
            for j in 0..n {
                vec_h.push(G2_generator.mul(tau[j]));
            }
            vec_h
        };
        // generate V_prime
        let V_prime = G2_generator.mul(alpha);
        // return the output
        return SRS {
            n,
            m,
            matrix_H,
            vec_H,
            vec_V,
            V_prime,
        };
    }

    fn commit(&self, poly: &B) -> Commitment<E> {
        Commitment {
            C: E::G1::sum((0..self.srs.n)
                .map(|i| {
                    E::G1::msm_unchecked(
                        self.srs.matrix_H[i].as_slice(),
                        poly.partial_evaluations_over_boolean_domain(i).as_slice(),
                    )
                })
                .collect::<Vec<_>>()
                .iter()
            ).into_affine(),
            aux: (0..self.srs.n)
                .map(|i| {
                    E::G1::msm_unchecked(
                        self.srs.vec_H.as_slice(),
                        poly.partial_evaluations_over_boolean_domain(i).as_slice(),
                    )
                })
                .collect::<Vec<_>>(),
        }
    }

    fn open(&self, poly: &B, com: Commitment<E>, b: &U::Input) -> OpeningProof<E, U> {
        OpeningProof {
            vec_D: {
                let mut vec = Vec::new();
                for g in com.aux {
                    vec.push(g.into());
                }
                vec
            },
            f_star_poly: U::from_multilinear_polynomial(poly.partial_evaluation(b)),
        }
    }

    fn verify(&self,
              lagrange_x: &dyn Evaluable<E, Input=U::Input>,
              C: &Commitment<E>,
              proof: &OpeningProof<E, U>,
              b: &U::Input,
              c: &U::Input,
              y: &E::ScalarField,
    ) -> bool {
        // first condition
        let pairing_rhs = E::multi_pairing(proof.vec_D.clone(), &self.srs.vec_V);
        let pairing_lhs = E::pairing(&C.C, &self.srs.V_prime);
        // second condition

        let msm_lhs = E::G1::msm_unchecked(&self.srs.vec_H, proof
            .f_star_poly
            .evaluations_over_boolean_domain().as_slice(),
        );
        let l_b = lagrange_x.evaluate(b);
        let msm_rhs = E::G1::msm_unchecked(proof.vec_D.as_slice(), &l_b);

        // third condition
        let y_expected = proof.f_star_poly.evaluate(c);
        // checking all three conditions
        return (pairing_lhs == pairing_rhs) && (msm_lhs == msm_rhs) && (y_expected == *y);
    }
}

#[cfg(test)]
pub mod test {
    use std::cmp::min;

    use ark_ec::pairing::Pairing;
    use ark_std::UniformRand;
    use rand::thread_rng;

    use crate::constant_for_curves::{E, ScalarField};
    use crate::polynomial::multilinear_polynomial::bivariate_multilinear::BivariateMultiLinearPolynomial;
    use crate::polynomial::multilinear_polynomial::dense_multilinear_poly::MultilinearPolynomial;
    use crate::polynomial::multilinear_polynomial::eq_poly::EqPolynomial;
    use crate::polynomial::traits::OneDimensionalPolynomial;
    use crate::polynomial_commitment::multilinear_pcs::{PolyCommit, PolyCommitTrait, SRS};

    #[test]
    fn test_setup() {
        let m = 4usize;
        let n = 4usize;
        let srs: SRS<E> = PolyCommit::<
            E,
            MultilinearPolynomial<<E as Pairing>::ScalarField, E>,
            BivariateMultiLinearPolynomial<<E as Pairing>::ScalarField, E>
        >::setup(n, m, &mut thread_rng());

        // asserting the sizes
        assert_eq!(srs.m, m);
        assert_eq!(srs.n, n);
        assert_eq!(srs.vec_H.len(), m);
        assert_eq!(srs.vec_V.len(), n);
        assert_eq!(srs.matrix_H.len(), n);
        assert_eq!(srs.matrix_H[0].len(), m);
        // checking pairing equalities
        // e(H[j, i], V[i]) = e(G_i^{tau_j}, V^{tau_i}) = e(H[i, i], V[j])
        for i in 0..min(m, n) {
            for j in 0..min(m, n) {
                let p1 = E::pairing(srs.matrix_H[j][i], srs.vec_V[i]);
                let p2 = E::pairing(srs.matrix_H[i][i], srs.vec_V[j]);
                assert_eq!(p1, p2);
            }
        }
    }

    #[test]
    fn test_end_to_end() {
        let n = 4usize;
        let m = 16usize;
        let srs: SRS<E> = PolyCommit::<
            E,
            MultilinearPolynomial<<E as Pairing>::ScalarField, E>,
            BivariateMultiLinearPolynomial<<E as Pairing>::ScalarField, E>
        >::setup(n, m, &mut thread_rng());

        // define the polynomial commitment
        let poly_commit: PolyCommit<
            E,
            MultilinearPolynomial<<E as Pairing>::ScalarField, E>,
            BivariateMultiLinearPolynomial<<E as Pairing>::ScalarField, E>
        > = PolyCommit { srs, phantom_data: Default::default() };

        // random bivariate polynomial
        let polynomial = BivariateMultiLinearPolynomial::from_multilinear_to_bivariate_multilinear(
            MultilinearPolynomial::rand(2 + 4, &mut thread_rng()),
            n,
        );

        // random points and evaluation
        let b = vec![
            ScalarField::rand(&mut thread_rng()), ScalarField::rand(&mut thread_rng()),
        ];
        let c = vec![
            ScalarField::rand(&mut thread_rng()), ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()), ScalarField::rand(&mut thread_rng()),
        ];
        let concat = {
            let mut res = vec![];
            res.extend(b.clone());
            res.extend(c.clone());
            res
        };

        let y = polynomial.poly.evaluate(&concat);

        // commit to the polynomial
        let com = poly_commit.commit(&polynomial);

        // open the commitment
        let open = poly_commit.open(&polynomial, com.clone(), &b);

        // verify the proof
        assert!(poly_commit.verify(&EqPolynomial::new(vec![]), &com, &open, &b, &c, &y));
    }
}

