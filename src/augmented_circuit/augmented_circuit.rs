#![allow(dead_code)]

use crate::accumulation_circuit::instance_circuit::AccumulatorInstanceVar;
use crate::accumulation_circuit::verifier_circuit::{AccumulatorVerifier, AccumulatorVerifierVar};
use crate::commitment::CommitmentScheme;
use crate::gadgets::non_native::non_native_affine_var::NonNativeAffineVar;
use crate::nexus_spartan::matrix_evaluation_accumulation::verifier_circuit::{MatrixEvaluationAccVerifier, MatrixEvaluationAccVerifierVar};
use crate::nexus_spartan::partial_verifier::partial_verifier::SpartanPartialVerifier;
use crate::nexus_spartan::partial_verifier::partial_verifier_var::SpartanPartialVerifierVar;
use crate::nova::cycle_fold::coprocessor_constraints::RelaxedOvaInstanceVar;
use crate::pcs::kzh2::split_between_x_and_y;
use crate::transcript::transcript_var::TranscriptVar;
use ark_crypto_primitives::sponge::Absorb;
use ark_ec::pairing::Pairing;
use ark_ec::short_weierstrass::{Affine, Projective, SWCurveConfig};
use ark_ff::PrimeField;
use ark_r1cs_std::alloc::{AllocVar, AllocationMode};
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::{ConstraintSystemRef, Namespace, SynthesisError};
use itertools::izip;
use std::borrow::Borrow;
use rand::thread_rng;
use crate::hash::poseidon::PoseidonHashVar;

const WITNESS_BLOAT: usize = 2;

type Output<'a, G2, C2, G1, F> = (
    (RelaxedOvaInstanceVar<G2, C2>, &'a AccumulatorInstanceVar<G1>),  // accumulator final instance, Ova final instance
    (Vec<FpVar<F>>, Vec<FpVar<F>>), // r_x, r_y
    (Vec<FpVar<F>>, Vec<FpVar<F>>, (FpVar<F>, FpVar<F>, FpVar<F>)), // (vector_x, vector_y, evaluations)
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AugmentedCircuit<G1, G2, C2, E, F>
where
    G1: SWCurveConfig + Clone,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField + Absorb,
    G2: SWCurveConfig<BaseField=F> + Clone,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>>,
    G1: SWCurveConfig<BaseField=G2::ScalarField, ScalarField=G2::BaseField>,
    E: Pairing<G1Affine=Affine<G1>, ScalarField=F>,
    F: PrimeField,
{
    pub spartan_partial_verifier: SpartanPartialVerifier<F, E>,
    pub kzh_acc_verifier: AccumulatorVerifier<G1, G2, C2, E>,
    pub matrix_evaluation_verifier: MatrixEvaluationAccVerifier<F>,
}

pub struct AugmentedCircuitVar<G1, G2, C2, F>
where
    F: PrimeField + Absorb,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField,
    G2: SWCurveConfig<BaseField=F> + Clone,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>>,
    G1: SWCurveConfig<BaseField=G2::ScalarField, ScalarField=G2::BaseField> + Clone,
{
    pub spartan_partial_verifier: SpartanPartialVerifierVar<F, G1>,
    pub kzh_acc_verifier: AccumulatorVerifierVar<G1, G2, C2>,
    pub matrix_evaluation_verifier: MatrixEvaluationAccVerifierVar<F>,
}

impl<G1, G2, C2, E, F> AllocVar<AugmentedCircuit<G1, G2, C2, E, F>, F> for AugmentedCircuitVar<G1, G2, C2, F>
where
    G1: SWCurveConfig + Clone,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField + Absorb,
    G2: SWCurveConfig<BaseField=F> + Clone,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>>,
    G1: SWCurveConfig<BaseField=G2::ScalarField, ScalarField=G2::BaseField>,
    E: Pairing<G1Affine=Affine<G1>, ScalarField=F>,
    F: PrimeField,
{
    fn new_variable<T: Borrow<AugmentedCircuit<G1, G2, C2, E, F>>>(
        cs: impl Into<Namespace<F>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        // Convert to Namespace<F>
        let ns = cs.into();
        // Get the constraint system reference
        let cs = ns.cs();

        // Fetch the instance of `AugmentedCircuit<F>`
        let binding = f()?;
        let data = binding.borrow();

        // Allocate the Spartan partial verifier
        let spartan_partial_verifier = SpartanPartialVerifierVar::new_variable(
            cs.clone(),
            || Ok(&data.spartan_partial_verifier),
            mode,
        )?;

        // Allocate the accumulator verifier
        let kzh_acc_verifier = AccumulatorVerifierVar::new_variable(
            cs.clone(),
            || Ok(&data.kzh_acc_verifier),
            mode,
        )?;

        // Allocate the accumulator verifier
        let matrix_evaluation_verifier = MatrixEvaluationAccVerifierVar::new_variable(
            cs.clone(),
            || Ok(&data.matrix_evaluation_verifier),
            mode,
        )?;

        Ok(AugmentedCircuitVar {
            spartan_partial_verifier,
            kzh_acc_verifier,
            matrix_evaluation_verifier,
        })
    }
}

impl<G1, G2, C2, F> AugmentedCircuitVar<G1, G2, C2, F>
where
    F: PrimeField + Absorb,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField,
    G2: SWCurveConfig<BaseField=F> + Clone,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>>,
    G1: SWCurveConfig<BaseField=G2::ScalarField, ScalarField=G2::BaseField> + Clone,
{
    fn verify<E: Pairing>(&self, cs: ConstraintSystemRef<F>, transcript: &mut TranscriptVar<F>) -> Output<G2, C2, G1, F> {
        let (rx, ry) = self.spartan_partial_verifier.verify(transcript);
        let (final_cycle_fold_instance, final_accumulator_instance) = self.kzh_acc_verifier.accumulate(transcript);

        // also return these later
        let ((vector_x, vector_y), evaluations) = self.matrix_evaluation_verifier.accumulate(transcript);

        // ************* do the consistency checks *************
        let length_x = self.kzh_acc_verifier.current_accumulator_instance_var.x_var.len();
        let length_y = self.kzh_acc_verifier.current_accumulator_instance_var.y_var.len();

        let (expected_x_var, expected_y_var) = split_between_x_and_y(length_x, length_y, &ry[1..], FpVar::zero());
        for (e1, e2) in izip!(&self.kzh_acc_verifier.current_accumulator_instance_var.x_var, expected_x_var) {
            e1.enforce_equal(&e2).expect("error while enforcing equality");
        }

        for (e1, e2) in izip!(&self.kzh_acc_verifier.current_accumulator_instance_var.y_var, expected_y_var) {
            e1.enforce_equal(&e2).expect("error while enforcing equality");
        }

        // enforce equal eval_Z_at_ry and accumulator.z_var
        self.spartan_partial_verifier.eval_vars_at_ry.enforce_equal(
            &self.kzh_acc_verifier
                .current_accumulator_instance_var
                .z_var
        ).expect("error while enforcing equality");

        // enforce the commitment in spartan verifier and the accumulator new instance
        NonNativeAffineVar::enforce_equal(
            &self.spartan_partial_verifier.instance.1,
            &self.kzh_acc_verifier.current_accumulator_instance_var.C_var,
        ).expect("error while enforcing equality");

        // pad it with some random poseidon hash
        let mut hash = PoseidonHashVar::new(cs.clone());
        for _ in 0..WITNESS_BLOAT {
            // get a random element
            let r = FpVar::new_variable(cs.clone(), || Ok(F::rand(&mut thread_rng())), AllocationMode::Witness).unwrap();
            // update sponge with this random element
            hash.update_sponge(vec![r]);
            // output the hash
            let _ = hash.output();
        }

        ((final_cycle_fold_instance, final_accumulator_instance), (rx, ry), (vector_x, vector_y, evaluations))
    }
}


#[cfg(test)]
mod tests {
    use ark_ec::pairing::Pairing;
    use crate::accumulation::accumulator::Accumulator;
    use crate::accumulation_circuit::prover::AccumulatorVerifierCircuitProver;
    use crate::constant_for_curves::{ScalarField, C2, E, G1, G2};
    use crate::nexus_spartan::crr1cs::is_sat;
    use crate::nexus_spartan::crr1cs::produce_synthetic_crr1cs;
    use crate::nexus_spartan::crr1csproof::CRR1CSProof;
    use crate::nexus_spartan::crr1csproof::{CRR1CSInstance, CRR1CSKey, CRR1CSShape, CRR1CSWitness};
    use crate::nexus_spartan::polycommitments::{PolyCommitmentScheme, ToAffine};
    use crate::nova::cycle_fold::coprocessor::setup_shape;
    use crate::pcs::kzh2::{split_between_x_and_y, PCSEngine};
    use crate::pcs::kzh2::KZH2SRS;
    use crate::polynomial::multilinear_poly::multilinear_poly::MultilinearPolynomial;
    use crate::transcript::transcript::Transcript;
    use ark_serialize::CanonicalSerialize;
    use ark_ff::AdditiveGroup;
    use ark_r1cs_std::alloc::{AllocVar, AllocationMode};
    use ark_relations::r1cs::{ConstraintSystem, SynthesisMode};
    use ark_std::{end_timer, start_timer};
    use rand::thread_rng;
    use crate::accumulation_circuit::verifier_circuit::AccumulatorVerifierVar;
    use crate::augmented_circuit::augmented_circuit::{AugmentedCircuitVar, Output};
    use crate::math::Math;
    use crate::nexus_spartan::matrix_evaluation_accumulation::verifier_circuit::{MatrixEvaluationAccVerifier, MatrixEvaluationAccVerifierVar};
    use crate::nexus_spartan::partial_verifier::partial_verifier::SpartanPartialVerifier;
    use crate::nexus_spartan::partial_verifier::partial_verifier_var::SpartanPartialVerifierVar;
    use crate::transcript::transcript_var::TranscriptVar;

    type F = ScalarField;

    const POLY_SETUP: usize = 19;

    #[test]
    fn test_augmented_circuit() {
        test_augmented_circuit_helper();
    }

    /// Take as input `proof_i` and `running_accumulator_{i}` and produce `proof_{i+1}` and `running_accumulator_{i+1}`.
    fn test_augmented_circuit_helper() {
        let SRS: KZH2SRS<E> = MultilinearPolynomial::setup(POLY_SETUP, &mut thread_rng()).unwrap();

        //////////////////// Generate `proof_i`   ////////////////////

        let num_vars = 131072;
        let num_cons = num_vars;
        let num_inputs = 10;

        // this generates a new instance/witness for spartan as well as PCS parameters
        let (spartan_shape, instance, witness, gens) = produce_synthetic_crr1cs::<E, MultilinearPolynomial<F>>(num_cons, num_vars, num_inputs);
        assert!(is_sat(&spartan_shape, &instance, &witness, &gens.gens_r1cs_sat).unwrap());

        let (num_cons, num_vars, _num_inputs) = (
            spartan_shape.get_num_cons(),
            spartan_shape.get_num_vars(),
            spartan_shape.get_num_inputs(),
        );

        let pcs_srs = gens.gens_r1cs_sat.keys.ck.clone();
        let acc_srs = Accumulator::setup(pcs_srs.clone(), &mut thread_rng());

        let mut prover_transcript = Transcript::new(b"example");

        // Get `proof_i` and random evaluation point (r_x, r_y)
        let (spartan_proof, rx, ry) = CRR1CSProof::prove(
            &spartan_shape,
            &instance,
            witness,
            &gens.gens_r1cs_sat,
            &mut prover_transcript,
        );

        // Get A(r_x, r_y), B(r_x, r_y), C(r_x, r_y)
        let current_A_B_C_evaluations = spartan_shape.inst.inst.evaluate(&rx, &ry);

        //////////////////// Extract current accumulator from the Spartan proof ////////////////////
        // Hossein: This should be a function in the accumulator module

        // Get the KZH opening proof from the Spartan proof
        let opening_proof = spartan_proof.proof_eval_vars_at_ry.clone();

        // Commitment to witness polynomial
        let commitment_w = instance.comm_W.clone();

        // Get the evaluation point of the opening proof
        let (x, y) = split_between_x_and_y::<F>(pcs_srs.degree_x.log_2(), pcs_srs.degree_y.log_2(), &ry[1..], F::ZERO);

        // Sanity check: verify the opening proof
        PCSEngine::verify(
            &pcs_srs,
            &commitment_w,
            &opening_proof,
            x.as_slice(),
            y.as_slice(),
            &spartan_proof.eval_vars_at_ry,
        );

        // Get accumulator from the opening proof
        let acc_instance = Accumulator::new_accumulator_instance_from_fresh_kzh_instance(
            &acc_srs,
            &commitment_w.C,
            x.as_slice(),
            y.as_slice(),
            &spartan_proof.eval_vars_at_ry,
        );

        let acc_witness = Accumulator::new_accumulator_witness_from_fresh_kzh_witness(
            &acc_srs,
            opening_proof,
            x.as_slice(),
            y.as_slice(),
        );

        let current_acc = Accumulator::new(&acc_instance, &acc_witness);

        // Check that the accumulator is valid
        assert!(
            Accumulator::decide(
                &acc_srs,
                &current_acc,
            )
        );

        //////////////////// Compute a random `running_accumulator_{i} ////////////////////

        let running_acc = Accumulator::rand(&acc_srs, &mut thread_rng());

        //////////////////// Construct A,B,C matrix evaluation accumulation verifier circuit ////////////////////

        let matrix_eval_acc_verifier = MatrixEvaluationAccVerifier::random_from_eval_point(
            &spartan_shape,
            rx,
            ry,
            &mut thread_rng(),
        );

        //////////////////// Construct Spartan partial verifier circuit ////////////////////

        let mut prover_transcript = Transcript::new(b"example");
        let mut verifier_transcript = prover_transcript.clone();
        let verifier_transcript_clone = verifier_transcript.clone();
        let partial_verifier = SpartanPartialVerifier::initialise(
            &spartan_proof,
            num_vars,
            num_cons,
            (instance.input.assignment, {
                let com_w: <E as Pairing>::G1Affine = instance.comm_W.clone().to_affine();
                com_w
            }),
            &current_A_B_C_evaluations,
            &mut prover_transcript,
        );

        partial_verifier.verify(&mut verifier_transcript);

        //////////////////// Construct the KZH AccVerifier circuit ////////////////////

        // Here we accumulate `current_acc` with `running_acc`

        // Hossein: Do all this cyclefold stuff in AccumulatorVerifierCircuitProver::new().

        // the shape of the R1CS instance
        let cycle_fold_shape = setup_shape::<G1, G2>().unwrap();

        // get trivial running instance
        let (cycle_fold_running_instance, cycle_fold_running_witness) = AccumulatorVerifierCircuitProver::<G1, G2, C2, E, F>::get_trivial_cycle_fold_running_instance_witness(&cycle_fold_shape);

        // get commitment_pp
        let commitment_pp = AccumulatorVerifierCircuitProver::<G1, G2, C2, E, F>::get_commitment_pp(&cycle_fold_shape);


        let kzh_acc_verifier_prover: AccumulatorVerifierCircuitProver<G1, G2, C2, E, F> = AccumulatorVerifierCircuitProver::new(
            &acc_srs,
            commitment_pp,
            running_acc,
            current_acc.clone(),
            cycle_fold_running_instance,
            cycle_fold_running_witness,
            prover_transcript,
        );

        // assert it's formated correctly
        kzh_acc_verifier_prover.is_satisfied();

        //////////////////// Construct the augmented circuit: piece everything together! ////////////////////
        // Hossein: There should be a function for all this

        let cs = ConstraintSystem::<F>::new_ref();

        println!("number of constraints after padding and before add augmented circuit: {}", cs.num_constraints());

        // Hossein: Why is this allocated as input and the ABC one as witness?
        let partial_verifier_var = SpartanPartialVerifierVar::new_variable(
            cs.clone(),
            || Ok(partial_verifier.clone()),
            AllocationMode::Input,
        ).unwrap();

        let acc_verifier_var = AccumulatorVerifierVar::<G1, G2, C2>::new::<E>(cs.clone(), kzh_acc_verifier_prover);

        let matrix_evaluation_verifier_var = MatrixEvaluationAccVerifierVar::new_variable(
            cs.clone(),
            || Ok(matrix_eval_acc_verifier.clone()),
            AllocationMode::Witness,
        ).unwrap();

        let augmented_circuit = AugmentedCircuitVar {
            spartan_partial_verifier: partial_verifier_var,
            kzh_acc_verifier: acc_verifier_var,
            matrix_evaluation_verifier: matrix_evaluation_verifier_var,
        };

        let mut transcript_var = TranscriptVar::from_transcript(cs.clone(), verifier_transcript_clone);

        let _output : Output<G2, C2, G1, F> = augmented_circuit.verify::<E>(cs.clone(), &mut transcript_var);

        assert!(cs.is_satisfied().unwrap());
        println!("augmented circuit constraints: {}", cs.num_constraints());

        // Set the mode to Prove before we convert it for spartan
        cs.set_mode(SynthesisMode::Prove { construct_matrices: true });
        cs.finalize();

        ////////// Now run the spartan prover on the augmented circuit /////////////////

        // convert to the corresponding Spartan types
        let shape = CRR1CSShape::<ScalarField>::convert::<G1>(cs.clone());
        let key: CRR1CSKey<E, MultilinearPolynomial<ScalarField>> = CRR1CSKey::new(&SRS, shape.get_num_cons(), shape.get_num_vars());
        // Commitment to w(x) happens here
        let instance: CRR1CSInstance<E, MultilinearPolynomial<ScalarField>> = CRR1CSInstance::convert(cs.clone(), &key.keys.ck);
        let witness = CRR1CSWitness::<ScalarField>::convert(cs.clone());

        // check that the Spartan instance-witness pair is still satisfying
        assert!(is_sat(&shape, &instance, &witness, &key).unwrap());

        let mut prover_transcript = Transcript::new(b"example");

        let (proof, rx, ry) = CRR1CSProof::prove(
            &shape,
            &instance,
            witness,
            &key,
            &mut prover_transcript,
        );

        println!("proof size: {}", proof.compressed_size());
        println!("acc size: {}", current_acc.compressed_size());

        //////////////////// Verifier ////////////////////

        let A_B_C_eval_timer = start_timer!(|| "ABC evals");
        // evaluate matrices A B C
        let inst_evals = shape.inst.inst.evaluate(&rx, &ry);
        end_timer!(A_B_C_eval_timer);

        let mut verifier_transcript = Transcript::new(b"example");
        let (num_cons, num_vars, _num_inputs) = (
            shape.get_num_cons(),
            shape.get_num_vars(),
            shape.get_num_inputs(),
        );

        let verify_timer = start_timer!(|| "verify");
        assert!(proof
            .verify(
                num_vars,
                num_cons,
                &instance,
                &inst_evals,
                &mut verifier_transcript,
            )
            .is_ok());
        end_timer!(verify_timer);
    }
}
