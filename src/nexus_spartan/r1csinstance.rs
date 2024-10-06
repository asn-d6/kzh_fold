use crate::math::Math;
use super::polycommitments::{PolyCommitmentScheme};
use super::sparse_mlpoly::{
    MultiSparseMatPolynomialAsDense, SparseMatEntry, SparseMatPolyCommitment,
    SparseMatPolyCommitmentKey, SparseMatPolynomial,
};
use super::timer::Timer;
use super::transcript::AppendToTranscript;
use ark_ec::pairing::Pairing;
use ark_ff::PrimeField;
use ark_serialize::*;
use ark_std::test_rng;
use merlin::Transcript;
use crate::polynomial::multilinear_poly::MultilinearPolynomial;

#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct R1CSInstance<F: PrimeField> {
    num_cons: usize,
    num_vars: usize,
    num_inputs: usize,
    pub(crate) A: SparseMatPolynomial<F>,
    pub(crate) B: SparseMatPolynomial<F>,
    pub(crate) C: SparseMatPolynomial<F>,
}


#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct R1CSCommitmentGens<E, PC>
where
    E: Pairing,
    PC: PolyCommitmentScheme<E>,
{
    gens: SparseMatPolyCommitmentKey<E, PC>,
}

impl<E: Pairing, PC: PolyCommitmentScheme<E>> R1CSCommitmentGens<E, PC> {
    pub fn new(
        SRS: &PC::SRS,
        num_cons: usize,
        num_vars: usize,
        num_inputs: usize,
        num_nz_entries: usize,
    ) -> R1CSCommitmentGens<E, PC> {
        assert!(num_inputs < num_vars);
        let num_poly_vars_x = num_cons.log_2();
        let num_poly_vars_y = (2 * num_vars).log_2();
        let gens =
            SparseMatPolyCommitmentKey::new(SRS, num_poly_vars_x, num_poly_vars_y, num_nz_entries, 3);
        R1CSCommitmentGens { gens }
    }
    pub fn get_min_num_vars(num_cons: usize, num_vars: usize, num_nz_entries: usize) -> usize {
        let num_poly_vars_x = num_cons.log_2();
        let num_poly_vars_y = (2 * num_vars).log_2();
        SparseMatPolyCommitmentKey::<E, PC>::get_min_num_vars(
            num_poly_vars_x,
            num_poly_vars_y,
            num_nz_entries,
            3,
        )
    }
}

#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct R1CSCommitment<E: Pairing, PC: PolyCommitmentScheme<E>> {
    num_cons: usize,
    num_vars: usize,
    num_inputs: usize,
    comm: SparseMatPolyCommitment<E, PC>,
}

impl<E: Pairing, PC: PolyCommitmentScheme<E>> AppendToTranscript<E> for R1CSCommitment<E, PC> {
    fn append_to_transcript(&self, _label: &'static [u8], transcript: &mut Transcript) {
        transcript.append_u64(b"num_cons", self.num_cons as u64);
        transcript.append_u64(b"num_vars", self.num_vars as u64);
        transcript.append_u64(b"num_inputs", self.num_inputs as u64);
        self.comm.append_to_transcript(b"comm", transcript);
    }
}

#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct R1CSDecommitment<F>
where
    F: Sync + CanonicalDeserialize + CanonicalSerialize + PrimeField,
{
    dense: MultiSparseMatPolynomialAsDense<F>,
}

impl<E: Pairing, PC: PolyCommitmentScheme<E>> R1CSCommitment<E, PC> {
    pub fn get_num_cons(&self) -> usize {
        self.num_cons
    }

    pub fn get_num_vars(&self) -> usize {
        self.num_vars
    }

    pub fn get_num_inputs(&self) -> usize {
        self.num_inputs
    }
}

impl<F: PrimeField> R1CSInstance<F> {
    pub fn new(
        num_cons: usize,
        num_vars: usize,
        num_inputs: usize,
        A: &[(usize, usize, F)],
        B: &[(usize, usize, F)],
        C: &[(usize, usize, F)],
    ) -> R1CSInstance<F> {
        Timer::print(&format!("number_of_constraints {}", num_cons));
        Timer::print(&format!("number_of_variables {}", num_vars));
        Timer::print(&format!("number_of_inputs {}", num_inputs));
        Timer::print(&format!("number_non-zero_entries_A {}", A.len()));
        Timer::print(&format!("number_non-zero_entries_B {}", B.len()));
        Timer::print(&format!("number_non-zero_entries_C {}", C.len()));

        // check that num_cons is a power of 2
        assert_eq!(num_cons.next_power_of_two(), num_cons);

        // check that num_vars is a power of 2
        assert_eq!(num_vars.next_power_of_two(), num_vars);

        // check that number_inputs + 1 <= num_vars
        assert!(num_inputs < num_vars);

        // no errors, so create polynomials
        let num_poly_vars_x = num_cons.log_2();
        let num_poly_vars_y = (2 * num_vars).log_2();

        let mat_A = (0..A.len())
            .map(|i| SparseMatEntry::new(A[i].0, A[i].1, A[i].2))
            .collect::<Vec<SparseMatEntry<F>>>();
        let mat_B = (0..B.len())
            .map(|i| SparseMatEntry::new(B[i].0, B[i].1, B[i].2))
            .collect::<Vec<SparseMatEntry<F>>>();
        let mat_C = (0..C.len())
            .map(|i| SparseMatEntry::new(C[i].0, C[i].1, C[i].2))
            .collect::<Vec<SparseMatEntry<F>>>();

        let poly_A = SparseMatPolynomial::new(num_poly_vars_x, num_poly_vars_y, mat_A);
        let poly_B = SparseMatPolynomial::new(num_poly_vars_x, num_poly_vars_y, mat_B);
        let poly_C = SparseMatPolynomial::new(num_poly_vars_x, num_poly_vars_y, mat_C);

        R1CSInstance {
            num_cons,
            num_vars,
            num_inputs,
            A: poly_A,
            B: poly_B,
            C: poly_C,
        }
    }

    pub fn get_num_vars(&self) -> usize {
        self.num_vars
    }

    pub fn get_num_cons(&self) -> usize {
        self.num_cons
    }

    pub fn get_num_inputs(&self) -> usize {
        self.num_inputs
    }

    pub fn produce_synthetic_r1cs(
        num_cons: usize,
        num_vars: usize,
        num_inputs: usize,
    ) -> (R1CSInstance<F>, Vec<F>, Vec<F>) {
        Timer::print(&format!("number_of_constraints {}", num_cons));
        Timer::print(&format!("number_of_variables {}", num_vars));
        Timer::print(&format!("number_of_inputs {}", num_inputs));

        let mut prng = test_rng();

        // assert num_cons and num_vars are power of 2
        assert_eq!((num_cons.log_2()).pow2(), num_cons);
        assert_eq!((num_vars.log_2()).pow2(), num_vars);

        // num_inputs + 1 <= num_vars
        assert!(num_inputs < num_vars);

        // z is organized as [vars,1,io]
        let size_z = num_vars + num_inputs + 1;

        // produce a random satisfying assignment
        let Z = {
            let mut Z: Vec<F> = (0..size_z).map(|_i| F::rand(&mut prng)).collect::<Vec<F>>();
            Z[num_vars] = F::one(); // set the constant term to 1
            Z
        };

        // three sparse matrices
        let mut A: Vec<SparseMatEntry<F>> = Vec::new();
        let mut B: Vec<SparseMatEntry<F>> = Vec::new();
        let mut C: Vec<SparseMatEntry<F>> = Vec::new();
        let one = F::one();
        for i in 0..num_cons {
            let A_idx = i % size_z;
            let B_idx = (i + 2) % size_z;
            A.push(SparseMatEntry::new(i, A_idx, one));
            B.push(SparseMatEntry::new(i, B_idx, one));
            let AB_val = Z[A_idx] * Z[B_idx];

            let C_idx = (i + 3) % size_z;
            let C_val = Z[C_idx];

            if C_val == F::zero() {
                C.push(SparseMatEntry::new(i, num_vars, AB_val));
            } else {
                C.push(SparseMatEntry::new(
                    i,
                    C_idx,
                    AB_val * C_val.inverse().unwrap(),
                ));
            }
        }

        Timer::print(&format!("number_non-zero_entries_A {}", A.len()));
        Timer::print(&format!("number_non-zero_entries_B {}", B.len()));
        Timer::print(&format!("number_non-zero_entries_C {}", C.len()));

        let num_poly_vars_x = num_cons.log_2();
        let num_poly_vars_y = (2 * num_vars).log_2();
        let poly_A = SparseMatPolynomial::new(num_poly_vars_x, num_poly_vars_y, A);
        let poly_B = SparseMatPolynomial::new(num_poly_vars_x, num_poly_vars_y, B);
        let poly_C = SparseMatPolynomial::new(num_poly_vars_x, num_poly_vars_y, C);

        let inst = R1CSInstance {
            num_cons,
            num_vars,
            num_inputs,
            A: poly_A,
            B: poly_B,
            C: poly_C,
        };

        assert!(inst.is_sat(&Z[..num_vars], &Z[num_vars + 1..]));

        (inst, Z[..num_vars].to_vec(), Z[num_vars + 1..].to_vec())
    }

    pub fn is_sat(&self, vars: &[F], input: &[F]) -> bool {
        assert_eq!(vars.len(), self.num_vars);
        assert_eq!(input.len(), self.num_inputs);

        let z = {
            let mut z = vars.to_vec();
            z.extend(&vec![F::one()]);
            z.extend(input);
            z
        };

        // verify if Az * Bz - Cz = [0...]
        let Az = self
            .A
            .multiply_vec(self.num_cons, self.num_vars + self.num_inputs + 1, &z);
        let Bz = self
            .B
            .multiply_vec(self.num_cons, self.num_vars + self.num_inputs + 1, &z);
        let Cz = self
            .C
            .multiply_vec(self.num_cons, self.num_vars + self.num_inputs + 1, &z);

        assert_eq!(Az.len(), self.num_cons);
        assert_eq!(Bz.len(), self.num_cons);
        assert_eq!(Cz.len(), self.num_cons);
        let res: usize = (0..self.num_cons)
            .map(|i| if Az[i] * Bz[i] == Cz[i] { 0 } else { 1 })
            .sum();

        res == 0
    }

    pub fn multiply_vec(
        &self,
        num_rows: usize,
        num_cols: usize,
        z: &[F],
    ) -> (MultilinearPolynomial<F>, MultilinearPolynomial<F>, MultilinearPolynomial<F>) {
        assert_eq!(num_rows, self.num_cons);
        assert_eq!(z.len(), num_cols);
        assert!(num_cols > self.num_vars);
        (
            MultilinearPolynomial::new(self.A.multiply_vec(num_rows, num_cols, z)),
            MultilinearPolynomial::new(self.B.multiply_vec(num_rows, num_cols, z)),
            MultilinearPolynomial::new(self.C.multiply_vec(num_rows, num_cols, z)),
        )
    }

    pub fn compute_eval_table_sparse(
        &self,
        num_rows: usize,
        num_cols: usize,
        evals: &[F],
    ) -> (Vec<F>, Vec<F>, Vec<F>) {
        assert_eq!(num_rows, self.num_cons);
        assert!(num_cols > self.num_vars);

        let evals_A = self.A.compute_eval_table_sparse(evals, num_rows, num_cols);
        let evals_B = self.B.compute_eval_table_sparse(evals, num_rows, num_cols);
        let evals_C = self.C.compute_eval_table_sparse(evals, num_rows, num_cols);

        (evals_A, evals_B, evals_C)
    }

    pub fn evaluate(&self, rx: &[F], ry: &[F]) -> (F, F, F) {
        let evals = SparseMatPolynomial::multi_evaluate(&[&self.A, &self.B, &self.C], rx, ry);
        (evals[0], evals[1], evals[2])
    }
}
