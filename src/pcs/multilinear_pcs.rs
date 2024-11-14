use ark_ec::AffineRepr;
use ark_ff::Zero;
use ark_serialize::Valid;
use std::ops::{Add, Mul};

use ark_ec::pairing::Pairing;
use ark_ec::{CurveGroup, VariableBaseMSM};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::UniformRand;
use derivative::Derivative;
use rand::RngCore;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::math::Math;
use crate::polynomial::eq_poly::eq_poly::EqPolynomial;
use crate::polynomial::multilinear_poly::multilinear_poly::MultilinearPolynomial;

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize, Derivative)]
pub struct PolynomialCommitmentSRS<E: Pairing> {
    /// degree_x = 2 ^ length of x variable
    pub degree_x: usize,
    /// degree_y = 2 ^ length of y variable
    pub degree_y: usize,
    pub matrix_H: Vec<Vec<E::G1Affine>>,
    pub vec_H: Vec<E::G1Affine>,
    pub vec_V: Vec<E::G2>,
    pub V_prime: E::G2,
}

#[derive(
    Default,
    Clone,
    Debug,
    PartialEq,
    Eq,
    CanonicalSerialize,
    CanonicalDeserialize,
    Derivative
)]
pub struct PCSCommitment<E: Pairing> {
    /// the commitment C to the polynomial
    pub C: E::G1Affine,
    /// auxiliary data which is in fact pederson commitments to rows of the polynomial
    pub aux: Vec<E::G1>,
}

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize, Derivative)]
pub struct PCSOpeningProof<E: Pairing> {
    pub vec_D: Vec<E::G1Affine>,
    pub f_star_poly: MultilinearPolynomial<E::ScalarField>,
}

/// Define the new struct that encapsulates the functionality of polynomial commitment
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize, Derivative)]
pub struct PCSEngine;

impl<E: Pairing> PolynomialCommitmentSRS<E> {
    pub fn get_x_length(&self) -> usize {
        self.degree_x.log_2()
    }

    pub fn get_y_length(&self) -> usize {
        self.degree_y.log_2()
    }
}

/// the function receives an input r and splits into two sub-vectors x and y to be used for PCS
/// It's used later when we have a constant SRS, and we pad the polynomial so we can commit to it via SRS
/// This function in fact pads to polynomial inputs by appends necessary zeros and split the input into x and y input
pub fn split_between_x_and_y<T: Clone>(x_length: usize, y_length: usize, r: &[T], zero: T) -> (Vec<T>, Vec<T>) {
    let total_length = x_length + y_length;

    // If r is smaller than the required length, extend it with zeros at the beginning
    let mut extended_r = r.to_vec();
    if r.len() < total_length {
        let mut zeros = vec![zero; total_length - r.len()];
        zeros.extend(extended_r);  // Prepend zeros to the beginning
        extended_r = zeros;
    }

    // Split the vector into two parts
    let r_x = extended_r[..x_length].to_vec();
    let r_y = extended_r[x_length..total_length].to_vec();

    (r_x, r_y)
}


/// all functions assume that poly size is already compatible with srs size, if not there's an interface that does padding
/// in the beginning in kzh.rs
impl PCSEngine {
    /// set up the PCS srs
    pub fn setup<T: RngCore, E: Pairing>(degree_x: usize, degree_y: usize, rng: &mut T) -> PolynomialCommitmentSRS<E> {
        // sample G_0, G_1, ..., G_m generators from group one
        let G1_generator_vec = {
            let mut elements = Vec::new();
            for _ in 0..degree_y {
                elements.push(E::G1Affine::rand(rng));
            }
            elements
        };

        // sample V, generator for group two
        let G2_generator = E::G2Affine::rand(rng);

        // sample trapdoors tau_0, tau_1, ..., tau_n, alpha
        let tau = {
            let mut elements = Vec::new();
            for _ in 0..degree_x {
                elements.push(E::ScalarField::rand(rng));
            }
            elements
        };

        let alpha = E::ScalarField::rand(rng);

        // generate matrix_H
        let matrix_H: Vec<Vec<_>> = (0..degree_x).into_par_iter()
            .map(|i| {
                let mut row = Vec::new();
                for j in 0..degree_y {
                    let g = G1_generator_vec[j].mul(tau[i]);
                    row.push(g.into());
                }
                row
            })
            .collect();

        // generate vec_H
        let vec_H = {
            let mut vec_h = Vec::new();
            for j in 0..degree_y {
                vec_h.push(G1_generator_vec[j].mul(alpha).into());
            }
            vec_h
        };

        // generate vec_V
        let vec_V = {
            let mut vec_h = Vec::new();
            for j in 0..degree_x {
                vec_h.push(G2_generator.mul(tau[j]));
            }
            vec_h
        };

        // generate V_prime
        let V_prime = G2_generator.mul(alpha);

        // return the output
        PolynomialCommitmentSRS {
            degree_x,
            degree_y,
            matrix_H,
            vec_H,
            vec_V,
            V_prime,
        }
    }

    pub fn commit<E: Pairing>(srs: &PolynomialCommitmentSRS<E>, poly: &MultilinearPolynomial<E::ScalarField>) -> PCSCommitment<E> {
        PCSCommitment {
            C: {
                // Collect all points and scalars into single vectors
                let mut base = Vec::new();
                let mut scalar = Vec::new();

                for i in 0..srs.degree_x {
                    // Collect points from matrix_H
                    base.extend_from_slice(srs.matrix_H[i].as_slice());
                    // Collect corresponding scalars from partial evaluations
                    scalar.extend_from_slice(poly.get_partial_evaluation_for_boolean_input(i, srs.degree_y).as_slice());
                }

                E::G1::msm_unchecked(&base, &scalar).into_affine()
            },
            aux: (0..srs.degree_x)
                .into_par_iter() // Parallelize the D^{(x)} computation
                .map(|i| {
                    E::G1::msm_unchecked(
                        srs.vec_H.as_slice(),
                        poly.get_partial_evaluation_for_boolean_input(i, srs.degree_y).as_slice(),
                    )
                })
                .collect::<Vec<_>>(),
        }
    }

    /// Creates a KZH proof for p(x,y) = z.
    /// This function does not actually need y, so we only get the left half of the eval point.
    pub fn open<E: Pairing>(poly: &MultilinearPolynomial<E::ScalarField>, com: PCSCommitment<E>, x: &[E::ScalarField]) -> PCSOpeningProof<E> {
        PCSOpeningProof {
            vec_D: {
                let mut vec = Vec::new();
                for g in com.aux {
                    vec.push(g.into());
                }
                vec
            },
            f_star_poly: poly.partial_evaluation(x),
        }
    }

    pub fn verify<E: Pairing>(srs: &PolynomialCommitmentSRS<E>,
                              C: &PCSCommitment<E>,
                              proof: &PCSOpeningProof<E>,
                              x: &[E::ScalarField],
                              y: &[E::ScalarField],
                              z: &E::ScalarField,
    ) {
        // Step 1: pairing check
        // Combine the pairings into a single multi-pairing
        let mut g1_elems: Vec<E::G1Affine> = Vec::with_capacity(1 + proof.vec_D.len());
        g1_elems.push(C.C.clone());
        for g1 in &proof.vec_D {
            let g1_neg: E::G1Affine = (E::G1Affine::zero() - g1).into();
            g1_elems.push(g1_neg);
        }

        let mut g2_elems = Vec::with_capacity(1 + srs.vec_V.len());
        g2_elems.push(srs.V_prime.clone());
        g2_elems.extend_from_slice(&srs.vec_V);

        // Perform the combined pairing check
        let pairing_product = E::multi_pairing(&g1_elems, &g2_elems);
        pairing_product.check().unwrap();

        // Step 2: MSM check
        // Combine the two MSMs into one
        let mut negated_eq_evals = EqPolynomial::new(x.to_vec()).evals();
        for scalar in &mut negated_eq_evals {
            *scalar = -*scalar;
        }

        let mut scalars = Vec::with_capacity(
            proof.f_star_poly.evaluation_over_boolean_hypercube.len() + negated_eq_evals.len(),
        );
        scalars.extend_from_slice(&proof.f_star_poly.evaluation_over_boolean_hypercube);
        scalars.extend_from_slice(&negated_eq_evals);

        let mut bases = Vec::with_capacity(srs.vec_H.len() + proof.vec_D.len());
        bases.extend_from_slice(&srs.vec_H);
        bases.extend_from_slice(&proof.vec_D);

        let msm_result = E::G1::msm_unchecked(&bases, &scalars);
        assert!(msm_result.is_zero());


        // Step 3: complete poly eval
        let y_expected = proof.f_star_poly.evaluate(y);
        assert_eq!(y_expected, *z);
    }
}


impl<E: Pairing> PCSCommitment<E> {
    /// Scales the commitment and its auxiliary elements by a scalar `r`
    pub fn scale_by_r(&mut self, r: &E::ScalarField) {
        // Scale the main commitment C by r
        let scaled_C = self.C.mul(r); // G1Affine -> G1Projective when multiplied by scalar

        // Scale each element in the aux vector by r
        let scaled_aux: Vec<E::G1> = self.aux.iter()
            .map(|element| element.mul(r))  // Multiply each element in aux by r
            .collect();

        // Update the commitment with the scaled values
        self.C = scaled_C.into_affine();  // Convert back to G1Affine after multiplication
        self.aux = scaled_aux;
    }
}


impl<E: Pairing> Add for PCSCommitment<E> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        // Ensure both commitments have the same size in aux vectors
        assert_eq!(self.aux.len(), other.aux.len(), "Aux vectors must have the same length");

        // Add the main commitment points C
        let new_C = (self.C + other.C).into_affine();

        // Add the corresponding elements in the aux vector
        let new_aux: Vec<E::G1> = self.aux.iter()
            .zip(other.aux.iter())
            .map(|(a, b)| *a + *b)
            .collect();

        // Return a new Commitment with the resulting sums
        PCSCommitment {
            C: new_C,
            aux: new_aux,
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::cmp::min;

    use ark_ec::pairing::Pairing;
    use ark_ff::AdditiveGroup;
    use ark_serialize::CanonicalSerialize;
    use ark_std::UniformRand;
    use rand::thread_rng;

    use crate::constant_for_curves::{ScalarField, E};
    use crate::pcs::multilinear_pcs::{split_between_x_and_y, PCSEngine, PolynomialCommitmentSRS};
    use crate::polynomial::multilinear_poly::multilinear_poly::MultilinearPolynomial;

    #[test]
    fn test_setup() {
        let degree_y = 4usize;
        let degree_x = 4usize;
        let srs: PolynomialCommitmentSRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());

        // asserting the sizes
        assert_eq!(srs.degree_y, degree_y);
        assert_eq!(srs.degree_x, degree_x);
        assert_eq!(srs.vec_H.len(), degree_y);
        assert_eq!(srs.vec_V.len(), degree_x);
        assert_eq!(srs.matrix_H.len(), degree_x);
        assert_eq!(srs.matrix_H[0].len(), degree_y);

        // checking pairing equalities
        // e(H[j, i], V[i]) = e(G_i^{tau_j}, V^{tau_i}) = e(H[i, i], V[j])
        for i in 0..min(degree_y, degree_x) {
            for j in 0..min(degree_y, degree_x) {
                let p1 = E::pairing(srs.matrix_H[j][i], srs.vec_V[i]);
                let p2 = E::pairing(srs.matrix_H[i][i], srs.vec_V[j]);
                assert_eq!(p1, p2);
            }
        }
    }

    #[test]
    fn test_end_to_end() {
        let (degree_x, degree_y) = (8usize, 32usize);
        let srs: PolynomialCommitmentSRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());

        // testing srs functions
        assert_eq!(3, srs.get_x_length());
        assert_eq!(5, srs.get_y_length());

        // ********************** test the input padding function split_between_x_and_y **********************
        let mut r = vec![
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
        ];
        r.extend(vec![ScalarField::ZERO; 3]);
        let x = r[0..3].to_vec();
        let y = r[3..].to_vec();

        // do the split and assert equality
        let length_x = srs.get_x_length();
        let length_y = srs.get_y_length();
        let (x_new, y_new) = split_between_x_and_y::<ScalarField>(
            length_x,
            length_y,
            r.as_slice(),
            ScalarField::ZERO,
        );
        assert_eq!(x_new, x);
        assert_eq!(y_new, y);

        // ********************** test the input padding function split_between_x_and_y **********************

        // random bivariate polynomial
        let polynomial = MultilinearPolynomial::rand(3 + 5, &mut thread_rng());

        // random points and evaluation
        let x = vec![
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
        ];
        let y = vec![
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
            ScalarField::rand(&mut thread_rng()),
        ];
        // concat inputs x and y, to evaluate the function
        let input = {
            let mut res = vec![];
            res.extend(x.clone());
            res.extend(y.clone());
            res
        };

        let z = polynomial.evaluate(&input);

        // commit to the polynomial
        let com = PCSEngine::commit(&srs, &polynomial);

        // open the commitment
        let open = PCSEngine::open(&polynomial, com.clone(), &x);

        // re compute x and y verify the proof
        PCSEngine::verify(&srs, &com, &open, &x, &y, &z);
    }

    /// Given f(x) and g(x) and their KZH commitments F and G.
    /// This test computes p(x) = f(x) + r * g(x),
    /// and checks that its commitment is P = F + r*G
    ///
    /// Prover sends F,G
    /// Verifier responds with r, rho
    /// Prover sends p(rho), f(rho), g(rho), proof_P_at_rho
    /// Verifier checks that p(rho) = f(rho) + r * g(rho)
    /// and the proof verifies using P = F + r * G
    #[test]
    fn test_homomorphism() {
        let degree_x = 16usize;
        let degree_y = 16usize;
        let num_vars = 8; // degree_x.log_2() + degree_y.log_2()

        let srs: PolynomialCommitmentSRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());

        let f_x: MultilinearPolynomial<ScalarField> = MultilinearPolynomial::rand(num_vars, &mut thread_rng());
        let g_x: MultilinearPolynomial<ScalarField> = MultilinearPolynomial::rand(num_vars, &mut thread_rng());

        let F = PCSEngine::commit(&srs, &f_x);
        let G = PCSEngine::commit(&srs, &g_x);

        // Verifier's challenge: for poly batching
        let r = ScalarField::rand(&mut thread_rng());
        // Verifier's challenge: evaluation point
        let rho = vec![ScalarField::rand(&mut thread_rng()); num_vars];
        // Split rho in half
        assert_eq!(rho.len() % 2, 0);
        let mid = rho.len() / 2;
        let (rho_first_half, rho_second_half) = rho.split_at(mid);

        // Compute p(x) = f(x) + r * g(x)
        let mut r_times_g_x = g_x.clone();
        r_times_g_x.scalar_mul(&r);
        let p_x = f_x.clone() + r_times_g_x;
        let P = PCSEngine::commit(&srs, &p_x);

        // Open p_x at rho
        let proof_P_at_rho = PCSEngine::open(&p_x, P.clone(), &rho_first_half);
        let p_at_rho = p_x.evaluate(&rho);

        // Verifier:
        assert_eq!(p_at_rho, f_x.evaluate(&rho) + r * g_x.evaluate(&rho));

        // Verifier: compute P = F + r*G
        let mut r_times_G = G.clone();
        r_times_G.scale_by_r(&r);
        let P_verifier = F + r_times_G;

        PCSEngine::verify(&srs, &P_verifier, &proof_P_at_rho, rho_first_half, rho_second_half, &p_at_rho);
    }

    #[test]
    fn count_witness() {
        let degrees = vec![(4, 4), (8, 8), (16, 16), (32, 32), (64, 64)];
        for (degree_x, degree_y) in degrees {
            let srs: PolynomialCommitmentSRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());
            // random bivariate polynomial
            let polynomial = MultilinearPolynomial::rand(
                srs.get_x_length() + srs.get_y_length(),
                &mut thread_rng(),
            );
            let com = PCSEngine::commit(&srs, &polynomial);

            // random points and evaluation
            let x = {
                let mut res = Vec::new();
                for _ in 0..srs.get_x_length() {
                    res.push(ScalarField::rand(&mut thread_rng()));
                }
                res
            };

            let open = PCSEngine::open(&polynomial, com.clone(), &x);
            let degree = degree_x * degree_y;
            println!("witness length in bytes: {} for degree {degree}",
                     open.compressed_size()
            );
        }
    }
}

