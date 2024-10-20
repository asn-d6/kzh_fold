use ark_crypto_primitives::sponge::Absorb;
use ark_ec::{CurveConfig, CurveGroup};
use ark_ec::pairing::Pairing;
use ark_ec::short_weierstrass::{Affine, Projective, SWCurveConfig};
use ark_ff::Field;
use ark_ff::PrimeField;
use rand::thread_rng;
use crate::accumulation::accumulator::{AccInstance, AccSRS, Accumulator};
use crate::accumulation_circuit::affine_to_projective;
use crate::commitment::CommitmentScheme;
use crate::gadgets::non_native::util::convert_field_one_to_field_two;
use crate::gadgets::r1cs::{OvaInstance, OvaWitness, R1CSShape, RelaxedOvaInstance, RelaxedOvaWitness};
use crate::gadgets::r1cs::ova::commit_T;
use crate::hash::pederson::PedersenCommitment;
use crate::nova::cycle_fold::coprocessor::{SecondaryCircuit, setup_shape, synthesize};

#[derive(Clone, Debug)]
pub struct AccumulatorVerifierCircuitProver<G1, G2, C2, E>
where
    G1: SWCurveConfig<BaseField=G2::ScalarField, ScalarField=G2::BaseField> + Clone,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField,
    G2: SWCurveConfig,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>, PP = Vec<Affine<G2>>>,
    E: Pairing<G1Affine=Affine<G1>, ScalarField=G1::ScalarField>,
{
    /// the randomness used for taking linear combination, it should be input from Accumulator::compute_fiat_shammir_challenge()
    pub beta: G1::ScalarField,

    /// srs for the accumulation
    pub srs: AccSRS<E>,

    /// the instance to be folded
    pub current_accumulator: Accumulator<E>,
    /// the running accumulator
    pub running_accumulator: Accumulator<E>,

    /// running cycle fold instance
    pub shape: R1CSShape<G2>,
    pub commitment_pp: <C2 as CommitmentScheme<Projective<G2>>>::PP,
    pub running_cycle_fold_instance: RelaxedOvaInstance<G2, C2>,
    pub running_cycle_fold_witness: RelaxedOvaWitness<G2>,

    // these are constant values
    pub n: u32,
    pub m: u32,
}

impl<G1, G2, C2, E> AccumulatorVerifierCircuitProver<G1, G2, C2, E>
where
    G1: SWCurveConfig<BaseField=G2::ScalarField, ScalarField=G2::BaseField> + Clone,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField,
    G2: SWCurveConfig,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>, PP = Vec<Affine<G2>>>,
    E: Pairing<G1Affine=Affine<G1>, ScalarField=G1::ScalarField>,
{
    #[inline(always)]
    pub fn get_current_acc_instance(&self) -> &AccInstance<E> {
        &self.current_accumulator.instance
    }

    #[inline(always)]
    pub fn get_running_acc_instance(&self) -> &AccInstance<E> {
        &self.running_accumulator.instance
    }
}

impl<G1, G2, C2, E> AccumulatorVerifierCircuitProver<G1, G2, C2, E>
where
    G1: SWCurveConfig + Clone,
    G1::BaseField: PrimeField,
    G1::ScalarField: PrimeField,
    G2: SWCurveConfig,
    G2::BaseField: PrimeField,
    C2: CommitmentScheme<Projective<G2>, PP = Vec<Affine<G2>>>,
    G1: SWCurveConfig<BaseField=<G2 as CurveConfig>::ScalarField, ScalarField=<G2 as CurveConfig>::BaseField>,
    E: Pairing<G1Affine=Affine<G1>, ScalarField=<G1 as CurveConfig>::ScalarField>,
    G2::BaseField: Absorb,
    G2::ScalarField: Absorb,
{
    pub fn is_satisfied(&self)
    where
        <G2 as CurveConfig>::ScalarField: Absorb,
    {
        assert!(Accumulator::decide(&self.srs, &self.running_accumulator));
        assert!(Accumulator::decide(&self.srs, &self.current_accumulator));
        self.shape.is_relaxed_ova_satisfied(
            &self.running_cycle_fold_instance,
            &self.running_cycle_fold_witness,
            &self.commitment_pp,
        ).expect("panic!");
    }

    pub fn compute_auxiliary_input_C(&self) -> (OvaInstance<G2, C2>, OvaWitness<G2>) {
        let g1 = affine_to_projective(self.running_accumulator.instance.C.clone());
        let g2 = affine_to_projective(self.current_accumulator.instance.C.clone());

        // C'' = beta * acc_running.instance.C + (1 - beta) * acc_instance.instance.C
        let g_out = (g1 * self.beta) + (g2 * (G1::ScalarField::ONE - self.beta));

        synthesize::<G1, G2, C2>(SecondaryCircuit {
            g1,
            g2,
            g_out,
            r: convert_field_one_to_field_two::<G1::ScalarField, G1::BaseField>(self.beta),
            flag: false,
        }, &self.commitment_pp[0..self.shape.num_vars].to_vec()).unwrap()
    }

    pub fn compute_auxiliary_input_T(&self) -> (OvaInstance<G2, C2>, OvaWitness<G2>) {
        let g1 = affine_to_projective(self.running_accumulator.instance.T.clone());
        let g2 = affine_to_projective(self.current_accumulator.instance.T.clone());

        // T'' = beta * acc_running.instance.T + (1 - beta) * acc_instance.instance.T
        let g_out = (g1 * self.beta) + (g2 * (G1::ScalarField::ONE - self.beta));

        synthesize::<G1, G2, C2>(SecondaryCircuit {
            g1,
            g2,
            g_out,
            r: convert_field_one_to_field_two::<G1::ScalarField, G1::BaseField>(self.beta),
            flag: false,
        }, &self.commitment_pp[0..self.shape.num_vars].to_vec()).unwrap()
    }

    pub fn compute_auxiliary_input_E_1(&self) -> (OvaInstance<G2, C2>, OvaWitness<G2>) {
        let g1 = affine_to_projective(self.running_accumulator.instance.E.clone());
        let g2 = affine_to_projective(self.current_accumulator.instance.E.clone());

        // E_temp = beta * acc_running.instance.E + (1 - beta) * acc_instance.instance.E
        let g_out = (g1 * self.beta) + (g2 * (G1::ScalarField::ONE - self.beta));

        synthesize::<G1, G2, C2>(SecondaryCircuit {
            g1,
            g2,
            g_out,
            r: convert_field_one_to_field_two::<G1::ScalarField, G1::BaseField>(self.beta),
            flag: false,
        }, &self.commitment_pp[0..self.shape.num_vars].to_vec()).unwrap()
    }

    pub fn compute_auxiliary_input_E_2(&self) -> (OvaInstance<G2, C2>, OvaWitness<G2>) {
        let e1 = affine_to_projective(self.running_accumulator.instance.E.clone());
        let e2 = affine_to_projective(self.current_accumulator.instance.E.clone());

        // E_temp = beta * e1 + (1 - beta) * e2
        let E_temp = (e1 * self.beta) + (e2 * (G1::ScalarField::ONE - self.beta));
        let Q = self.compute_proof_Q();
        let g_out = E_temp + Q * (self.beta * (G1::ScalarField::ONE - self.beta));

        synthesize::<G1, G2, C2>(SecondaryCircuit {
            g1: Q,
            g2: E_temp,
            g_out,
            r: convert_field_one_to_field_two::<G1::ScalarField, G1::BaseField>(self.beta * (G1::ScalarField::ONE - self.beta)),
            flag: true,
        }, &self.commitment_pp[0..self.shape.num_vars].to_vec()).unwrap()
    }

    pub fn compute_proof_Q(&self) -> Projective<G1> {
        // since acc_instance takes (1- beta) then it should be first in the function argument
        affine_to_projective(Accumulator::prove(&self.srs, &self.current_accumulator, &self.running_accumulator).2)
    }

    pub fn compute_result_accumulator_instance(&self) -> AccInstance<E> {
        Accumulator::prove(&self.srs, &self.current_accumulator, &self.running_accumulator).0
    }

    pub fn compute_cycle_fold_proofs_and_final_instance(&self) -> (
        C2::Commitment,
        C2::Commitment,
        C2::Commitment,
        C2::Commitment,
        RelaxedOvaInstance<G2, C2>
    ) {
        let compute_commit_and_fold =
            |running_witness: &RelaxedOvaWitness<G2>,
             running_instance: &RelaxedOvaInstance<G2, C2>,
             witness: &OvaWitness<G2>,
             instance: &OvaInstance<G2, C2>,
             beta: &G2::ScalarField,
            | -> (C2::Commitment, RelaxedOvaWitness<G2>, RelaxedOvaInstance<G2, C2>) {
                let (T, com_T) = commit_T(
                    &self.shape,
                    &self.commitment_pp,
                    running_instance,
                    running_witness,
                    instance,
                    witness,
                ).unwrap();

                // Fold the running instance and witness with the first proof
                let new_running_instance = running_instance.fold(instance, &com_T, beta).unwrap();
                let new_running_witness = running_witness.fold(witness, &T, beta).unwrap();

                (com_T, new_running_witness, new_running_instance)
            };

        let beta_non_native = convert_field_one_to_field_two::<G1::ScalarField, G1::BaseField>(self.beta);

        // first fold auxiliary_input_C with the running instance
        let (instance_C, witness_C) = self.compute_auxiliary_input_C();
        let (com_C, new_running_witness, new_running_instance) = compute_commit_and_fold(
            &self.running_cycle_fold_witness,
            &self.running_cycle_fold_instance,
            &witness_C,
            &instance_C,
            &beta_non_native,
        );

        self.shape.is_ova_satisfied(&instance_C, &witness_C, &self.commitment_pp).unwrap();
        self.shape.is_relaxed_ova_satisfied(&new_running_instance, &new_running_witness, &self.commitment_pp).unwrap();


        // first fold auxiliary_input_T with the running instance
        let (instance_T, witness_T) = self.compute_auxiliary_input_T();
        let (com_T, new_running_witness, new_running_instance) = compute_commit_and_fold(
            &new_running_witness,
            &new_running_instance,
            &witness_T,
            &instance_T,
            &beta_non_native,
        );

        self.shape.is_ova_satisfied(&instance_T, &witness_T, &self.commitment_pp).unwrap();
        self.shape.is_relaxed_ova_satisfied(&new_running_instance, &new_running_witness, &self.commitment_pp).unwrap();

        // first fold auxiliary_input_E_1 with the running instance
        let (instance_E_1, witness_E_1) = self.compute_auxiliary_input_E_1();
        let (com_E_1, new_running_witness, new_running_instance) = compute_commit_and_fold(
            &new_running_witness,
            &new_running_instance,
            &witness_E_1,
            &instance_E_1,
            &beta_non_native,
        );

        self.shape.is_ova_satisfied(&instance_E_1, &witness_E_1, &self.commitment_pp).unwrap();
        self.shape.is_relaxed_ova_satisfied(&new_running_instance, &new_running_witness, &self.commitment_pp).unwrap();

        // first fold auxiliary_input_E_1 with the running instance
        let (instance_E_2, witness_E_2) = self.compute_auxiliary_input_E_2();
        let (com_E_2, new_running_witness, new_running_instance) = compute_commit_and_fold(
            &new_running_witness,
            &new_running_instance,
            &witness_E_2,
            &instance_E_2,
            &beta_non_native,
        );

        self.shape.is_ova_satisfied(&instance_E_2, &witness_E_2, &self.commitment_pp).unwrap();
        self.shape.is_relaxed_ova_satisfied(&new_running_instance, &new_running_witness, &self.commitment_pp).unwrap();

        (com_C, com_T, com_E_1, com_E_2, new_running_instance)
    }

    pub fn rand(srs: &AccSRS<E>) -> AccumulatorVerifierCircuitProver<G1, G2, C2, E> {
        // build an instance of AccInstanceCircuit
        let current_accumulator = Accumulator::random_satisfying_accumulator(&srs, &mut thread_rng());
        let running_accumulator = Accumulator::random_satisfying_accumulator(&srs, &mut thread_rng());

        // compute Q
        let Q = Accumulator::helper_function_Q(&srs, &current_accumulator, &running_accumulator);

        // the shape of the R1CS instance
        let shape = setup_shape::<G1, G2>().unwrap();

        // public parameters of Pedersen
        let commitment_pp: Vec<Affine<G2>> = PedersenCommitment::<Projective<G2>>::setup(shape.num_vars + shape.num_constraints, b"test", &());

        let cycle_fold_running_instance = RelaxedOvaInstance::new(&shape);
        let cycle_fold_running_witness = RelaxedOvaWitness::zero(&shape);

        let beta = Accumulator::compute_fiat_shamir_challenge(srs, &current_accumulator.instance, &running_accumulator.instance, Q);

        AccumulatorVerifierCircuitProver {
            beta,
            srs: srs.clone(),
            current_accumulator,
            running_accumulator,
            shape,
            commitment_pp,
            running_cycle_fold_instance: cycle_fold_running_instance,
            running_cycle_fold_witness: cycle_fold_running_witness,
            n: srs.pc_srs.degree_x as u32,
            m: srs.pc_srs.degree_y as u32,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use ark_ec::CurveConfig;
    use ark_ff::Field;
    use rand::thread_rng;

    use crate::accumulation::accumulator::Accumulator;
    use crate::accumulation_circuit::prover::AccumulatorVerifierCircuitProver;
    use crate::commitment::CommitmentScheme;
    use crate::constant_for_curves::{BaseField, E, G1, G2, ScalarField};
    use crate::gadgets::non_native::util::convert_field_one_to_field_two;
    use crate::hash::pederson::PedersenCommitment;
    use crate::pcs::multilinear_pcs::{PolyCommit, SRS};

    type GrumpkinCurveGroup = ark_grumpkin::Projective;
    type C2 = PedersenCommitment<GrumpkinCurveGroup>;


    #[test]
    pub fn random_instance_is_satisfying() {
        // specifying degrees of polynomials
        let (n, m) = (4, 4);

        // get a random srs
        let srs = {
            let srs_pcs: SRS<E> = PolyCommit::<E>::setup(n, m, &mut thread_rng());
            Accumulator::setup(srs_pcs.clone(), &mut thread_rng())
        };

        let prover: AccumulatorVerifierCircuitProver<G1, G2, C2, E> = AccumulatorVerifierCircuitProver::rand(&srs);
        prover.is_satisfied();
    }

    #[test]
    pub fn auxiliary_input_C_correctness() {
        // specifying degrees of polynomials
        let (n, m) = (4, 4);

        // get a random srs
        let srs = {
            let srs_pcs: SRS<E> = PolyCommit::<E>::setup(n, m, &mut thread_rng());
            Accumulator::setup(srs_pcs.clone(), &mut thread_rng())
        };

        let prover: AccumulatorVerifierCircuitProver<G1, G2, C2, E> = AccumulatorVerifierCircuitProver::rand(&srs);

        let (r1cs_instance, _) = prover.compute_auxiliary_input_C();
        let secondary_circuit = r1cs_instance.parse_secondary_io().unwrap();

        // get the accumulated result
        let new_acc_instance = Accumulator::prove(&prover.srs, &prover.current_accumulator, &prover.running_accumulator).0;

        assert_eq!(secondary_circuit.r, convert_field_one_to_field_two::<ScalarField, BaseField>(prover.beta));
        assert_eq!(secondary_circuit.flag, false);
        assert_eq!(secondary_circuit.g1, prover.running_accumulator.instance.C);
        assert_eq!(secondary_circuit.g2, prover.current_accumulator.instance.C);
        assert_eq!(secondary_circuit.g_out, new_acc_instance.C);
    }

    #[test]
    pub fn auxiliary_input_T_correctness() {
        // specifying degrees of polynomials
        let (n, m) = (4, 4);

        // get a random srs
        let srs = {
            let srs_pcs: SRS<E> = PolyCommit::<E>::setup(n, m, &mut thread_rng());
            Accumulator::setup(srs_pcs.clone(), &mut thread_rng())
        };

        let prover: AccumulatorVerifierCircuitProver<G1, G2, C2, E> = AccumulatorVerifierCircuitProver::rand(&srs);

        let (r1cs_instance, _) = prover.compute_auxiliary_input_T();
        let secondary_circuit = r1cs_instance.parse_secondary_io().unwrap();

        // get the accumulated result
        let new_acc_instance = Accumulator::prove(&prover.srs, &prover.current_accumulator, &prover.running_accumulator).0;

        assert_eq!(secondary_circuit.r, convert_field_one_to_field_two::<ScalarField, BaseField>(prover.beta));
        assert_eq!(secondary_circuit.flag, false);
        assert_eq!(secondary_circuit.g1, prover.running_accumulator.instance.T);
        assert_eq!(secondary_circuit.g2, prover.current_accumulator.instance.T);
        assert_eq!(secondary_circuit.g_out, new_acc_instance.T);
    }


    #[test]
    pub fn auxiliary_input_E_correctness() {
        // specifying degrees of polynomials
        let (n, m) = (4, 4);

        // get a random srs
        let srs = {
            let srs_pcs: SRS<E> = PolyCommit::<E>::setup(n, m, &mut thread_rng());
            Accumulator::setup(srs_pcs.clone(), &mut thread_rng())
        };

        let prover: AccumulatorVerifierCircuitProver<G1, G2, C2, E> = AccumulatorVerifierCircuitProver::rand(&srs);

        let (r1cs_instance, _) = prover.compute_auxiliary_input_E_1();
        let secondary_circuit_E_1 = r1cs_instance.parse_secondary_io().unwrap();

        let (r1cs_instance, _) = prover.compute_auxiliary_input_E_2();
        let secondary_circuit_E_2 = r1cs_instance.parse_secondary_io().unwrap();

        let Q = prover.compute_proof_Q();

        // get the accumulated result
        let new_acc_instance = Accumulator::prove(&prover.srs, &prover.current_accumulator, &prover.running_accumulator).0;

        // checking correctness of flags
        assert_eq!(secondary_circuit_E_1.flag, false);
        assert_eq!(secondary_circuit_E_2.flag, true);

        // checking correctness of randomness
        assert_eq!(secondary_circuit_E_1.r, convert_field_one_to_field_two::<ScalarField, BaseField>(prover.beta));
        assert_eq!(secondary_circuit_E_2.r, convert_field_one_to_field_two::<ScalarField, BaseField>(prover.beta * (ScalarField::ONE - prover.beta)));

        // check E_temp is present in two circuits
        assert_eq!(secondary_circuit_E_1.g_out, secondary_circuit_E_2.g2);

        // check input to the first circuit is correct
        assert_eq!(secondary_circuit_E_1.g1, prover.running_accumulator.instance.E);
        assert_eq!(secondary_circuit_E_1.g2, prover.current_accumulator.instance.E);

        // check input to the first circuit is correct
        assert_eq!(secondary_circuit_E_2.g1, Q);
    }

    #[test]
    pub fn compute_cycle_fold_proofs_correctness() {
        // specifying degrees of polynomials
        let (n, m) = (4, 4);

        // get a random srs
        let srs = {
            let srs_pcs: SRS<E> = PolyCommit::<E>::setup(n, m, &mut thread_rng());
            Accumulator::setup(srs_pcs.clone(), &mut thread_rng())
        };

        let prover: AccumulatorVerifierCircuitProver<G1, G2, C2, E> = AccumulatorVerifierCircuitProver::rand(&srs);
        let _ = prover.compute_cycle_fold_proofs_and_final_instance();
    }
}

