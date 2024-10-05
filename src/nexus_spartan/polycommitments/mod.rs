use core::fmt::Debug;

use ark_ec::pairing::Pairing;
use ark_poly_commit::Error;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    ops::{Add, AddAssign, Mul, MulAssign},
    rand::RngCore,
};
use derivative::Derivative;
use merlin::Transcript;

use crate::nexus_spartan::transcript::AppendToTranscript;
use crate::polynomial::multilinear_poly::MultilinearPolynomial;

pub mod error;

pub trait VectorCommitmentScheme<E: Pairing> {
    type VectorCommitment: AppendToTranscript<E>
    + Sized
    + Sync
    + CanonicalSerialize
    + CanonicalDeserialize;
    type CommitmentKey;
    fn commit(vec: &[E::ScalarField], ck: &Self::CommitmentKey) -> Self::VectorCommitment;

    // Commitment to the zero vector of length n
    fn zero(n: usize) -> Self::VectorCommitment;
}

pub trait PolyCommitmentTrait<E: Pairing>:
Sized
+ AppendToTranscript<E>
+ Debug
+ CanonicalSerialize
+ CanonicalDeserialize
+ PartialEq
+ Eq
+ Default
+ Clone
{
    // this should be the commitment to the zero vector of length n
    fn zero(n: usize) -> Self;
}


#[derive(CanonicalSerialize, CanonicalDeserialize, Derivative, Debug)]
#[derivative(Clone(bound = ""))]
pub struct PCSKeys<E, PC>
where
    PC: PolyCommitmentScheme<E> + ?Sized,
    E: Pairing,
{
    pub ck: PC::PolyCommitmentKey,
    pub vk: PC::EvalVerifierKey,
}

pub trait PolyCommitmentScheme<E: Pairing>: Send + Sync {
    type SRS: CanonicalSerialize + CanonicalDeserialize + Clone;
    type PolyCommitmentKey: CanonicalSerialize + CanonicalDeserialize + Clone;
    type EvalVerifierKey: CanonicalSerialize + CanonicalDeserialize + Clone;
    type Commitment: PolyCommitmentTrait<E>;
    // The commitments should be compatible with a homomorphic vector commitment valued in G
    type PolyCommitmentProof: Sync + CanonicalSerialize + CanonicalDeserialize + Debug;

    // Optionally takes `vector_comm` as a "hint" to speed up the commitment process if a
    // commitment to the vector of evaluations has already been computed
    fn commit(
        poly: &MultilinearPolynomial<E::ScalarField>,
        ck: &Self::PolyCommitmentKey,
    ) -> Self::Commitment;

    fn prove(
        C: Option<&Self::Commitment>,
        poly: &MultilinearPolynomial<E::ScalarField>,
        r: &[E::ScalarField],
        eval: &E::ScalarField,
        ck: &Self::PolyCommitmentKey,
        transcript: &mut Transcript,
    ) -> Self::PolyCommitmentProof;

    fn verify(
        commitment: &Self::Commitment,
        proof: &Self::PolyCommitmentProof,
        ck: &Self::EvalVerifierKey,
        transcript: &mut Transcript,
        r: &[E::ScalarField],
        eval: &E::ScalarField,
    ) -> Result<(), error::PCSError>;

    // Generate a SRS using the provided RNG; this is just for testing purposes, since in reality
    // we need to perform a trusted setup ceremony and then read the SRS from a file.
    fn setup(
        max_poly_vars: usize,
        label: &'static [u8],
        rng: &mut impl RngCore,
    ) -> Result<Self::SRS, Error>;

    fn trim(srs: &Self::SRS, supported_num_vars: usize) -> PCSKeys<E, Self>;
}

impl<E: Pairing, PC: PolyCommitmentScheme<E>> VectorCommitmentScheme<E> for PC {
    type VectorCommitment = PC::Commitment;
    type CommitmentKey = PC::PolyCommitmentKey;
    fn commit(vec: &[<E>::ScalarField], ck: &Self::CommitmentKey) -> Self::VectorCommitment {
        let poly = MultilinearPolynomial::new(vec.to_vec());
        PC::commit(&poly, ck)
    }
    fn zero(n: usize) -> Self::VectorCommitment {
        PC::Commitment::zero(n)
    }
}