use std::borrow::Borrow;
use ark_crypto_primitives::sponge::Absorb;
use ark_ff::PrimeField;
use ark_r1cs_std::alloc::{AllocVar, AllocationMode};
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::fields::FieldVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::R1CSVar;
use ark_r1cs_std::select::CondSelectGadget;
use ark_relations::ns;
use ark_relations::r1cs::{ConstraintSystemRef, Namespace, SynthesisError};
use crate::nexus_spartan::sparse_polynomial::sparse_polynomial::SparsePoly;
use crate::nexus_spartan::sparse_polynomial::uint32::Unsigned32Var;

pub struct SparsePolyVar<F: Absorb + PrimeField> {
    num_vars: usize,
    evaluations: Vec<FpVar<F>>,
}

impl<F: PrimeField + Absorb> SparsePolyVar<F> {
    pub fn new(num_vars: usize, evaluations: Vec<FpVar<F>>) -> SparsePolyVar<F> {
        SparsePolyVar {
            num_vars,
            evaluations,
        }
    }
}

// Implement AllocVar for SparsePoly
impl<F: PrimeField + Absorb> AllocVar<SparsePoly<F>, F> for SparsePolyVar<F> {
    fn new_variable<T: Borrow<SparsePoly<F>>>(
        cs: impl Into<Namespace<F>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        // Convert to Namespace<F>
        let ns = cs.into();
        // Get the constraint system reference
        let cs = ns.cs();

        let res = f();
        let sparse_poly = res.as_ref().map(|e| e.borrow()).map_err(|err| *err);

        // Allocate each field element in the `evaluations` as circuit variables
        let mut evaluation_var = Vec::new();
        for i in 0..sparse_poly?.evaluations.len() {
            evaluation_var.push(FpVar::new_variable(
                ns!(cs, "y"),
                || Ok(sparse_poly?.evaluations[i]),
                mode,
            )?);
        }

        // Return the allocated SparsePolyVar
        Ok(SparsePolyVar {
            num_vars: sparse_poly?.num_vars,
            evaluations: evaluation_var,
        })
    }
}

impl<F: PrimeField + Absorb> R1CSVar<F> for SparsePolyVar<F> {
    type Value = SparsePoly<F>;

    // Return the constraint system associated with the variable
    fn cs(&self) -> ConstraintSystemRef<F> {
        self.evaluations
            .iter()
            .fold(ConstraintSystemRef::None, |cs, eval| cs.or(eval.cs()))
    }

    // Extract the underlying values of the evals and return a SparsePoly<F>
    fn value(&self) -> Result<Self::Value, SynthesisError> {
        // Retrieve the values of the evals (they are Option<F> inside FpVar<F>)
        let evals = self
            .evaluations
            .iter()
            .map(|eval_var| eval_var.value()) // Extract value for each eval_var
            .collect::<Result<Vec<_>, SynthesisError>>()?; // Collect them into a Vec<F>

        // Return a new SparsePoly<F> with the same number of variables
        Ok(SparsePoly {
            num_vars: self.num_vars,
            evaluations: evals,
        })
    }
}

impl<F: PrimeField + Absorb> SparsePolyVar<F> {
    pub fn compute_chi(a: &[Boolean<F>], r: &[FpVar<F>]) -> FpVar<F> {
        assert_eq!(a.len(), r.len(), "Unequal length vectors");
        let mut chi_i = FpVar::one();
        for j in 0..r.len() {
            let val = FpVar::conditionally_select(&a[j], &r[j], &(FpVar::one() - &r[j])).unwrap();
            chi_i = chi_i * val;
        }
        chi_i
    }

    /// It's currently using the optimised version Unsigned32Var which takes 60 constraints
    /// to for each call of get_bits_canonical_order which FpVar counterpart takes 600 constraints
    pub fn evaluate(&self, r: &[FpVar<F>]) -> FpVar<F> {
        assert_eq!(self.num_vars, r.len());
        let mut res = FpVar::<F>::zero();
        let cs = r[0].cs();
        let mut counter = Unsigned32Var::new(cs);
        for i in 0..self.evaluations.len() {
            let bits = counter.get_bits_canonical_order(r.len());
            counter.increment_inplace();
            res += SparsePolyVar::compute_chi(&bits, r) * &self.evaluations[i];
        }
        res
    }
}
