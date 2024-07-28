use std::borrow::Cow;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Div, Mul};

use ark_crypto_primitives::Error;
use ark_ec::{AffineRepr, CurveGroup, ScalarMul, VariableBaseMSM};
use ark_ec::pairing::Pairing;
use ark_ff::{AdditiveGroup, One, PrimeField, Zero};
use ark_poly::DenseUVPolynomial;
use ark_std::{end_timer, start_timer, UniformRand};
use rand::RngCore;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq
)]
pub struct UniversalParams<E: Pairing> {
    /// Group elements of the form `{ \beta^i G }`, where `i` ranges from 0 to `degree`.
    pub powers_of_g: Vec<E::G1Affine>,
    /// Group elements of the form `{ \beta^i \gamma G }`, where `i` ranges from 0 to `degree`.
    pub powers_of_gamma_g: BTreeMap<usize, E::G1Affine>,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// \beta times the above generator of G2.
    pub beta_h: E::G2Affine,
    /// Group elements of the form `{ \beta^i G2 }`, where `i` ranges from `0` to `-degree`.
    pub neg_powers_of_h: BTreeMap<usize, E::G2Affine>,
    /// The generator of G2, prepared for use in pairings.
    pub prepared_h: E::G2Prepared,
    /// \beta times the above generator of G2, prepared for use in pairings.
    pub prepared_beta_h: E::G2Prepared,
}

#[derive(
    Default,
    Hash,
    Clone,
    Debug,
    PartialEq
)]
pub struct Powers<'a, E: Pairing> {
    /// Group elements of the form `β^i G`, for different values of `i`.
    pub powers_of_g: Cow<'a, [E::G1Affine]>,
    /// Group elements of the form `β^i γG`, for different values of `i`.
    pub powers_of_gamma_g: Cow<'a, [E::G1Affine]>,
}

#[derive(
    Default,
    Clone,
    Debug,
    PartialEq,
    Eq
)]
pub struct VerifierKey<E: Pairing> {
    /// The generator of G1.
    pub g: E::G1Affine,
    /// The generator of G1 that is used for making a commitment hiding.
    pub gamma_g: E::G1Affine,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// \beta times the above generator of G2.
    pub beta_h: E::G2Affine,
    /// The generator of G2, prepared for use in pairings.
    pub prepared_h: E::G2Prepared,
    /// \beta times the above generator of G2, prepared for use in pairings.
    pub prepared_beta_h: E::G2Prepared,
}

#[derive(Default, Clone, Debug)]
pub struct PreparedVerifierKey<E: Pairing> {
    /// The generator of G1, prepared for power series.
    pub prepared_g: Vec<E::G1Affine>,
    /// The generator of G2, prepared for use in pairings.
    pub prepared_h: E::G2Prepared,
    /// \beta times the above generator of G2, prepared for use in pairings.
    pub prepared_beta_h: E::G2Prepared,
}

impl<E: Pairing> PreparedVerifierKey<E> {
    /// prepare `PreparedVerifierKey` from `VerifierKey`
    pub fn prepare(vk: &VerifierKey<E>) -> Self {
        let supported_bits = E::ScalarField::MODULUS_BIT_SIZE as usize;

        let mut prepared_g = Vec::<E::G1Affine>::new();
        let mut g = E::G1::from(vk.g.clone());
        for _ in 0..supported_bits {
            prepared_g.push(g.clone().into());
            g.double_in_place();
        }

        Self {
            prepared_g,
            prepared_h: vk.prepared_h.clone(),
            prepared_beta_h: vk.prepared_beta_h.clone(),
        }
    }
}

/// `Commitment` commits to a polynomial. It is output by `KZG10::commit`.
#[derive(
    Default,
    Hash,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq
)]
pub struct Commitment<E: Pairing>(
    /// The commitment is a group element.
    pub E::G1Affine,
);

/// `PreparedCommitment` commits to a polynomial and prepares for mul_bits.
#[derive(
    Default,
    Hash,
    Clone,
    Debug,
    PartialEq,
    Eq
)]
pub struct PreparedCommitment<E: Pairing>(
    /// The commitment is a group element.
    pub Vec<E::G1Affine>,
);

impl<E: Pairing> PreparedCommitment<E> {
    /// prepare `PreparedCommitment` from `Commitment`
    pub fn prepare(comm: &Commitment<E>) -> Self {
        let mut prepared_comm = Vec::<E::G1Affine>::new();
        let mut cur = E::G1::from(comm.0.clone());

        let supported_bits = E::ScalarField::MODULUS_BIT_SIZE as usize;

        for _ in 0..supported_bits {
            prepared_comm.push(cur.clone().into());
            cur.double_in_place();
        }

        Self { 0: prepared_comm }
    }
}

/// `Randomness` hides the polynomial inside a commitment. It is output by `KZG10::commit`.
#[derive(
    Hash,
    Clone,
    Debug,
    PartialEq,
    Eq
)]
pub struct Randomness<F: PrimeField, P: DenseUVPolynomial<F>> {
    /// For KZG10, the commitment randomness is a random polynomial.
    pub blinding_polynomial: P,
    _field: PhantomData<F>,
}

impl<F: PrimeField, P: DenseUVPolynomial<F>> Randomness<F, P> {
    /// Does `self` provide any hiding properties to the corresponding commitment?
    /// `self.is_hiding() == true` only if the underlying polynomial is non-zero.
    #[inline]
    pub fn is_hiding(&self) -> bool {
        !self.blinding_polynomial.is_zero()
    }

    /// What is the degree of the hiding polynomial for a given hiding bound?
    #[inline]
    pub fn calculate_hiding_polynomial_degree(hiding_bound: usize) -> usize {
        hiding_bound + 1
    }
}

impl<F: PrimeField, P: DenseUVPolynomial<F>> Randomness<F, P> {
    fn empty() -> Self {
        Self {
            blinding_polynomial: P::zero(),
            _field: PhantomData,
        }
    }

    fn rand<R: RngCore>(hiding_bound: usize, _: bool, _: Option<usize>, rng: &mut R) -> Self {
        let mut randomness = Randomness::empty();
        let hiding_poly_degree = Self::calculate_hiding_polynomial_degree(hiding_bound);
        randomness.blinding_polynomial = P::rand(hiding_poly_degree, rng);
        randomness
    }
}

impl<'a, F: PrimeField, P: DenseUVPolynomial<F>> Add<&'a Randomness<F, P>> for Randomness<F, P> {
    type Output = Self;

    #[inline]
    fn add(mut self, other: &'a Self) -> Self {
        self.blinding_polynomial += &other.blinding_polynomial;
        self
    }
}

impl<'a, F: PrimeField, P: DenseUVPolynomial<F>> Add<(F, &'a Randomness<F, P>)>
for Randomness<F, P>
{
    type Output = Self;

    #[inline]
    fn add(mut self, other: (F, &'a Randomness<F, P>)) -> Self {
        self += other;
        self
    }
}

impl<'a, F: PrimeField, P: DenseUVPolynomial<F>> AddAssign<&'a Randomness<F, P>>
for Randomness<F, P>
{
    #[inline]
    fn add_assign(&mut self, other: &'a Self) {
        self.blinding_polynomial += &other.blinding_polynomial;
    }
}

impl<'a, F: PrimeField, P: DenseUVPolynomial<F>> AddAssign<(F, &'a Randomness<F, P>)>
for Randomness<F, P>
{
    #[inline]
    fn add_assign(&mut self, (f, other): (F, &'a Randomness<F, P>)) {
        self.blinding_polynomial += (f, &other.blinding_polynomial);
    }
}


/// `Proof` is an evaluation proof that is output by `KZG10::open`.
#[derive(
    Default,
    Hash,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq
)]
pub struct Proof<E: Pairing> {
    /// This is a commitment to the witness polynomial; see [KZG10] for more details.
    pub w: E::G1Affine,
    /// This is the evaluation of the random polynomial at the point for which
    /// the evaluation proof was produced.
    pub random_v: Option<E::ScalarField>,
}

pub struct KZG10<E: Pairing, P: DenseUVPolynomial<E::ScalarField>> {
    _engine: PhantomData<E>,
    _poly: PhantomData<P>,
}

impl<E, P> KZG10<E, P>
where
    E: Pairing,
    P: DenseUVPolynomial<E::ScalarField, Point=E::ScalarField>,
    for<'a, 'b> &'a P: Div<&'b P, Output=P>,
{
    /// Constructs public parameters when given as input the maximum degree `degree`
    /// for the polynomial commitment scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// ```
    pub fn setup<R: RngCore>(
        max_degree: usize,
        produce_g2_powers: bool,
        rng: &mut R,
    ) -> Result<UniversalParams<E>, Error> {
        if max_degree < 1 {
            panic!("wrong degree")
        }
        let setup_time = start_timer!(|| format!("KZG10::Setup with degree {}", max_degree));
        let beta = E::ScalarField::rand(rng);
        let g = E::G1::rand(rng);
        let gamma_g = E::G1::rand(rng);
        let h = E::G2::rand(rng);

        // powers_of_beta = [1, b, ..., b^(max_degree + 1)], len = max_degree + 2
        let mut powers_of_beta = vec![E::ScalarField::one()];
        let mut cur = beta;
        for _ in 0..=max_degree {
            powers_of_beta.push(cur);
            cur *= &beta;
        }

        let g_time = start_timer!(|| "Generating powers of G");
        let powers_of_g = g.batch_mul(&powers_of_beta[0..max_degree + 1]);
        end_timer!(g_time);

        // Use the entire `powers_of_beta`, since we want to be able to support
        // up to D queries.
        let gamma_g_time = start_timer!(|| "Generating powers of gamma * G");
        let powers_of_gamma_g = gamma_g
            .batch_mul(&powers_of_beta)
            .into_iter()
            .enumerate()
            .collect();
        end_timer!(gamma_g_time);

        let neg_powers_of_h_time = start_timer!(|| "Generating negative powers of h in G2");
        let neg_powers_of_h = if produce_g2_powers {
            let mut neg_powers_of_beta = vec![E::ScalarField::one()];
            let mut cur = E::ScalarField::one() / &beta;
            for _ in 0..max_degree {
                neg_powers_of_beta.push(cur);
                cur /= &beta;
            }

            h.batch_mul(&neg_powers_of_beta)
                .into_iter()
                .enumerate()
                .collect()
        } else {
            BTreeMap::new()
        };

        end_timer!(neg_powers_of_h_time);

        let h = h.into_affine();
        let beta_h = h.mul(beta).into_affine();
        let prepared_h = h.into();
        let prepared_beta_h = beta_h.into();

        let pp = UniversalParams {
            powers_of_g,
            powers_of_gamma_g,
            h,
            beta_h,
            neg_powers_of_h,
            prepared_h,
            prepared_beta_h,
        };
        end_timer!(setup_time);
        Ok(pp)
    }

    /// Outputs a commitment to `polynomial`.
    ///
    /// # Examples
    ///
    /// ```
    /// ```
    pub fn commit(
        powers: &Powers<E>,
        polynomial: &P,
        hiding_bound: Option<usize>,
        rng: Option<&mut dyn RngCore>,
    ) -> Result<(Commitment<E>, Randomness<E::ScalarField, P>), Error> {
        let commit_time = start_timer!(|| format!(
            "Committing to polynomial of degree {} with hiding_bound: {:?}",
            polynomial.degree(),
            hiding_bound,
        ));

        let (num_leading_zeros, plain_coeffs) =
            skip_leading_zeros_and_convert_to_bigints(polynomial);

        let msm_time = start_timer!(|| "MSM to compute commitment to plaintext poly");
        let mut commitment = <E::G1 as VariableBaseMSM>::msm_bigint(
            &powers.powers_of_g[num_leading_zeros..],
            &plain_coeffs,
        );
        end_timer!(msm_time);

        let mut randomness = Randomness::<E::ScalarField, P>::empty();
        if let Some(hiding_degree) = hiding_bound {
            let mut rng = rng.unwrap();
            let sample_random_poly_time = start_timer!(|| format!(
                "Sampling a random polynomial of degree {}",
                hiding_degree
            ));

            randomness = Randomness::rand(hiding_degree, false, None, &mut rng);
            end_timer!(sample_random_poly_time);
        }

        let random_ints = convert_to_bigints(&randomness.blinding_polynomial.coeffs());
        let msm_time = start_timer!(|| "MSM to compute commitment to random poly");
        let random_commitment = <E::G1 as VariableBaseMSM>::msm_bigint(
            &powers.powers_of_gamma_g,
            random_ints.as_slice(),
        )
            .into_affine();
        end_timer!(msm_time);

        commitment += &random_commitment;

        end_timer!(commit_time);
        Ok((Commitment(commitment.into()), randomness))
    }

    /// Compute witness polynomial.
    ///
    /// The witness polynomial w(x) the quotient of the division (p(x) - p(z)) / (x - z)
    /// Observe that this quotient does not change with z because
    /// p(z) is the remainder term. We can therefore omit p(z) when computing the quotient.
    pub fn compute_witness_polynomial(
        p: &P,
        point: P::Point,
        randomness: &Randomness<E::ScalarField, P>,
    ) -> Result<(P, Option<P>), Error> {
        let divisor = P::from_coefficients_vec(vec![-point, E::ScalarField::one()]);

        let witness_time = start_timer!(|| "Computing witness polynomial");
        let witness_polynomial = p / &divisor;
        end_timer!(witness_time);

        let random_witness_polynomial = if randomness.is_hiding() {
            let random_p = &randomness.blinding_polynomial;

            let witness_time = start_timer!(|| "Computing random witness polynomial");
            let random_witness_polynomial = random_p / &divisor;
            end_timer!(witness_time);
            Some(random_witness_polynomial)
        } else {
            None
        };

        Ok((witness_polynomial, random_witness_polynomial))
    }

    pub(crate) fn open_with_witness_polynomial<'a>(
        powers: &Powers<E>,
        point: P::Point,
        randomness: &Randomness<E::ScalarField, P>,
        witness_polynomial: &P,
        hiding_witness_polynomial: Option<&P>,
    ) -> Result<Proof<E>, Error> {
        let (num_leading_zeros, witness_coeffs) =
            skip_leading_zeros_and_convert_to_bigints(witness_polynomial);

        let witness_comm_time = start_timer!(|| "Computing commitment to witness polynomial");
        let mut w = <E::G1 as VariableBaseMSM>::msm_bigint(
            &powers.powers_of_g[num_leading_zeros..],
            &witness_coeffs,
        );
        end_timer!(witness_comm_time);

        let random_v = if let Some(hiding_witness_polynomial) = hiding_witness_polynomial {
            let blinding_p = &randomness.blinding_polynomial;
            let blinding_eval_time = start_timer!(|| "Evaluating random polynomial");
            let blinding_evaluation = blinding_p.evaluate(&point);
            end_timer!(blinding_eval_time);

            let random_witness_coeffs = convert_to_bigints(&hiding_witness_polynomial.coeffs());
            let witness_comm_time =
                start_timer!(|| "Computing commitment to random witness polynomial");
            w += &<E::G1 as VariableBaseMSM>::msm_bigint(
                &powers.powers_of_gamma_g,
                &random_witness_coeffs,
            );
            end_timer!(witness_comm_time);
            Some(blinding_evaluation)
        } else {
            None
        };

        Ok(Proof {
            w: w.into_affine(),
            random_v,
        })
    }

    /// On input a polynomial `p` and a point `point`, outputs a proof for the same.
    pub fn open<'a>(
        powers: &Powers<E>,
        p: &P,
        point: P::Point,
        rand: &Randomness<E::ScalarField, P>,
    ) -> Result<Proof<E>, Error> {
        let open_time = start_timer!(|| format!("Opening polynomial of degree {}", p.degree()));

        let witness_time = start_timer!(|| "Computing witness polynomials");
        let (witness_poly, hiding_witness_poly) = Self::compute_witness_polynomial(p, point, rand)?;
        end_timer!(witness_time);

        let proof = Self::open_with_witness_polynomial(
            powers,
            point,
            rand,
            &witness_poly,
            hiding_witness_poly.as_ref(),
        );

        end_timer!(open_time);
        proof
    }

    /// Verifies that `value` is the evaluation at `point` of the polynomial
    /// committed inside `comm`.
    pub fn check(
        vk: &VerifierKey<E>,
        comm: &Commitment<E>,
        point: E::ScalarField,
        value: E::ScalarField,
        proof: &Proof<E>,
    ) -> Result<bool, Error> {
        let check_time = start_timer!(|| "Checking evaluation");
        let mut inner = comm.0.into_group() - &vk.g.mul(value);
        if let Some(random_v) = proof.random_v {
            inner -= &vk.gamma_g.mul(random_v);
        }
        let lhs = E::pairing(inner, vk.h);

        let inner = vk.beta_h.into_group() - &vk.h.mul(point);
        let rhs = E::pairing(proof.w, inner);

        end_timer!(check_time, || format!("Result: {}", lhs == rhs));
        Ok(lhs == rhs)
    }

    /// Check that each `proof_i` in `proofs` is a valid proof of evaluation for
    /// `commitment_i` at `point_i`.
    pub fn batch_check<R: RngCore>(
        vk: &VerifierKey<E>,
        commitments: &[Commitment<E>],
        points: &[E::ScalarField],
        values: &[E::ScalarField],
        proofs: &[Proof<E>],
        rng: &mut R,
    ) -> Result<bool, Error> {
        let check_time =
            start_timer!(|| format!("Checking {} evaluation proofs", commitments.len()));

        let mut total_c = <E::G1>::zero();
        let mut total_w = <E::G1>::zero();

        let combination_time = start_timer!(|| "Combining commitments and proofs");
        let mut randomizer = E::ScalarField::one();
        // Instead of multiplying g and gamma_g in each turn, we simply accumulate
        // their coefficients and perform a final multiplication at the end.
        let mut g_multiplier = E::ScalarField::zero();
        let mut gamma_g_multiplier = E::ScalarField::zero();
        for (((c, z), v), proof) in commitments.iter().zip(points).zip(values).zip(proofs) {
            let w = proof.w;
            let mut temp = w.mul(*z);
            temp += &c.0;
            let c = temp;
            g_multiplier += &(randomizer * v);
            if let Some(random_v) = proof.random_v {
                gamma_g_multiplier += &(randomizer * &random_v);
            }
            total_c += &c.mul(randomizer);
            total_w += &w.mul(randomizer);
            // We don't need to sample randomizers from the full field,
            // only from 128-bit strings.
            randomizer = u128::rand(rng).into();
        }
        total_c -= &vk.g.mul(g_multiplier);
        total_c -= &vk.gamma_g.mul(gamma_g_multiplier);
        end_timer!(combination_time);

        let to_affine_time = start_timer!(|| "Converting results to affine for pairing");
        let affine_points = E::G1::normalize_batch(&[-total_w, total_c]);
        let (total_w, total_c) = (affine_points[0], affine_points[1]);
        end_timer!(to_affine_time);

        let pairing_time = start_timer!(|| "Performing product of pairings");
        let result = E::multi_pairing(
            [total_w, total_c],
            [vk.prepared_beta_h.clone(), vk.prepared_h.clone()],
        )
            .0
            .is_one();
        end_timer!(pairing_time);
        end_timer!(check_time, || format!("Result: {}", result));
        Ok(result)
    }
}

fn skip_leading_zeros_and_convert_to_bigints<F: PrimeField, P: DenseUVPolynomial<F>>(
    p: &P,
) -> (usize, Vec<F::BigInt>) {
    let mut num_leading_zeros = 0;
    while num_leading_zeros < p.coeffs().len() && p.coeffs()[num_leading_zeros].is_zero() {
        num_leading_zeros += 1;
    }
    let coeffs = convert_to_bigints(&p.coeffs()[num_leading_zeros..]);
    (num_leading_zeros, coeffs)
}

fn convert_to_bigints<F: PrimeField>(p: &[F]) -> Vec<F::BigInt> {
    let to_bigint_time = start_timer!(|| "Converting polynomial coeffs to bigints");
    let coeffs = ark_std::cfg_iter!(p)
        .map(|s| s.into_bigint())
        .collect::<Vec<_>>();
    end_timer!(to_bigint_time);
    coeffs
}

#[cfg(test)]
mod tests {
    use ark_bn254::{Bn254, Fr};
    use ark_ec::pairing::Pairing;
    use ark_ff::Field;
    use ark_poly::{DenseUVPolynomial, Polynomial};
    use ark_poly::univariate::DensePolynomial;
    use ark_std::{test_rng, UniformRand};
    use rand::thread_rng;
    use crate::kzg::{KZG10, Powers, UniversalParams, VerifierKey};
    use crate::lagrange_basis::LagrangeBasis;

    type F = Fr;
    type E = Bn254;

    /// Specializes the public parameters for a given maximum degree `d` for polynomials
    /// `d` should be less that `pp.max_degree()`.
    pub(crate) fn trim(
        pp: &UniversalParams<E>,
        mut supported_degree: usize,
    ) -> (Powers<E>, VerifierKey<E>) {
        if supported_degree == 1 {
            supported_degree += 1;
        }
        let powers_of_g = pp.powers_of_g[..=supported_degree].to_vec();
        let powers_of_gamma_g = (0..=supported_degree)
            .map(|i| pp.powers_of_gamma_g[&i])
            .collect();

        let powers = Powers {
            powers_of_g: ark_std::borrow::Cow::Owned(powers_of_g),
            powers_of_gamma_g: ark_std::borrow::Cow::Owned(powers_of_gamma_g),
        };
        let vk = VerifierKey {
            g: pp.powers_of_g[0],
            gamma_g: pp.powers_of_gamma_g[&0],
            h: pp.h,
            beta_h: pp.beta_h,
            prepared_h: pp.prepared_h.clone(),
            prepared_beta_h: pp.prepared_beta_h.clone(),
        };
        (powers, vk)
    }


    #[test]
    pub fn kzg() {
        type Poly = DensePolynomial<<E as Pairing>::ScalarField>;

        // Set up public parameters
        let rng = &mut test_rng();
        let degree = 128 * 128;
        let params = KZG10::<E, Poly>::setup(degree, false, rng).expect("Setup failed");
        let (ck, vk) = trim(&params, degree);

        // Generate commitment
        let polynomial = Poly:: rand(degree, rng);
        let hiding_bound = Some(1);

        let (comm, r) = KZG10::<E, Poly>::commit(&ck, &polynomial, hiding_bound, Some(rng)).expect("Commitment failed");

        // Open commitment to get proof
        let point = F::rand(rng);
        let proof = KZG10::<E, Poly>::open(&ck, &polynomial, point, &r).expect("Proof generation failed");

        // Verify proof
        let value = polynomial.evaluate(&point);
        let is_valid = KZG10::<E, Poly>::check(&vk, &comm, point, value, &proof).expect("Verification failed");

        assert!(is_valid, "Proof verification failed");
    }
}