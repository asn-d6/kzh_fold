use std::borrow::Borrow;
use std::fmt::Debug;

use ark_crypto_primitives::sponge::constraints::AbsorbGadget;
use ark_ec::{AffineRepr, CurveConfig, CurveGroup};
use ark_ec::pairing::Pairing;
use ark_ec::short_weierstrass::{Affine, Projective, SWCurveConfig};
use ark_ff::{Field, One, PrimeField, Zero};
use ark_r1cs_std::alloc::{AllocationMode, AllocVar};
use ark_r1cs_std::fields::FieldVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::nonnative::NonNativeFieldVar;
use ark_r1cs_std::groups::curves::short_weierstrass::{AffineVar, ProjectiveVar};
use ark_r1cs_std::prelude::UInt8;
use ark_r1cs_std::R1CSVar;
use ark_r1cs_std::uint32::UInt32;
use ark_relations::ns;
use ark_relations::r1cs::{ConstraintSystemRef, Namespace, SynthesisError};

use crate::accumulation::accumulator::AccInstance;
use crate::accumulation_circuit::affine_to_projective;
use crate::gadgets::non_native::non_native_affine_var::NonNativeAffineVar;
use crate::gadgets::non_native::util::non_native_to_fpvar;

#[derive(Clone)]
/// the circuit is defined on scalar of G1
pub struct AccumulatorInstanceVar<G1>
where
    G1: SWCurveConfig + Clone,
    <G1 as CurveConfig>::ScalarField: PrimeField,
    <G1 as CurveConfig>::BaseField: PrimeField,
{
    // group points with base field G1::BaseField
    pub C_var: NonNativeAffineVar<G1>,
    pub T_var: NonNativeAffineVar<G1>,
    pub E_var: NonNativeAffineVar<G1>,

    // the field elements G1::ScalarField
    pub x_var: Vec<FpVar<G1::ScalarField>>,
    pub y_var: Vec<FpVar<G1::ScalarField>>,
    pub z_var: FpVar<G1::ScalarField>,
}


impl<G1> AccumulatorInstanceVar<G1>
where
    G1: SWCurveConfig + Clone,
    <G1 as CurveConfig>::ScalarField: PrimeField,
    <G1 as CurveConfig>::BaseField: PrimeField,
{
    pub(crate) fn cs(&self) -> ConstraintSystemRef<G1::ScalarField> {
        // assert the vectors have non-zero length
        assert_ne!(self.x_var.len(), 0, "vector x with length zero");
        assert_ne!(self.y_var.len(), 0, "vector y with length zero");

        // return the constraint system
        self.C_var.cs().or(self.T_var.cs())
            .or(self.E_var.cs())
            .or(self.x_var[0].cs())
            .or(self.y_var[0].cs())
            .or(self.z_var.cs())
    }

    pub(crate) fn value<E>(&self) -> Result<AccInstance<E>, SynthesisError>
    where
        E: Pairing<G1Affine=Affine<G1>, ScalarField=<G1 as CurveConfig>::ScalarField>,
    {
        Ok(AccInstance {
            C: self.C_var.value().unwrap().into(),
            T: self.T_var.value().unwrap().into(),
            E: self.E_var.value().unwrap().into(),
            x: self.x_var.clone()
                .into_iter()
                .map(|element| element.value().unwrap())
                .collect(),
            y: self.y_var.clone()
                .into_iter()
                .map(|element| element.value().unwrap())
                .collect(),
            z: self.z_var.value().unwrap(),
        })
    }
}


impl<G1, E> AllocVar<AccInstance<E>, <G1 as CurveConfig>::ScalarField> for AccumulatorInstanceVar<G1>
where
    G1: SWCurveConfig + Clone,
    <G1 as CurveConfig>::ScalarField: PrimeField,
    <G1 as CurveConfig>::BaseField: PrimeField,
    E: Pairing<G1Affine=Affine<G1>, ScalarField=<G1 as CurveConfig>::ScalarField>,
{
    fn new_variable<T: Borrow<AccInstance<E>>>(
        cs: impl Into<Namespace<<G1 as CurveConfig>::ScalarField>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let ns = cs.into();
        let cs = ns.cs();

        let res = f();
        let circuit = res.as_ref().map(|e| e.borrow()).map_err(|err| *err);

        let C_var = NonNativeAffineVar::new_variable(
            ns!(cs, "C"),
            || circuit.map(|e| affine_to_projective(e.C)),
            mode,
        ).unwrap();

        let T_var = NonNativeAffineVar::new_variable(
            ns!(cs, "T"),
            || circuit.map(|e| affine_to_projective(e.T)),
            mode,
        ).unwrap();

        let E_var = NonNativeAffineVar::new_variable(
            ns!(cs, "E"),
            || circuit.map(|e| affine_to_projective(e.E)),
            mode,
        ).unwrap();

        let x_var = {
            let mut res = Vec::new();
            for i in 0..circuit.unwrap().x.len() {
                res.push(FpVar::new_variable(
                    ns!(cs, "x"),
                    || circuit.map(|e| e.x[i]),
                    mode,
                ).unwrap());
            }
            res
        };

        let y_var = {
            let mut res = Vec::new();
            for i in 0..circuit.unwrap().y.len() {
                res.push(FpVar::new_variable(
                    ns!(cs, "y"),
                    || circuit.map(|e| e.y[i]),
                    mode,
                ).unwrap());
            }
            res
        };

        let z_var = FpVar::new_variable(
            ns!(cs, "z"),
            || circuit.map(|e| e.z),
            mode,
        ).unwrap();

        Ok(AccumulatorInstanceVar {
            C_var,
            T_var,
            E_var,
            x_var,
            y_var,
            z_var,
        })
    }
}

impl<G1> AbsorbGadget<G1::ScalarField> for AccumulatorInstanceVar<G1>
where
    G1: SWCurveConfig + Clone,
    <G1 as CurveConfig>::ScalarField: PrimeField,
    <G1 as CurveConfig>::BaseField: PrimeField,
{
    fn to_sponge_bytes(&self) -> Result<Vec<UInt8<G1::ScalarField>>, SynthesisError> {
        unreachable!()
    }

    // todo: remember this might cause some problem later
    fn to_sponge_field_elements(&self) -> Result<Vec<FpVar<G1::ScalarField>>, SynthesisError> {
        // Call to_sponge_field_elements on each NonNativeAffineVar
        let mut fpvar_vec = Vec::new();

        fpvar_vec.extend(self.C_var.to_sponge_field_elements()?);
        fpvar_vec.extend(self.T_var.to_sponge_field_elements()?);
        fpvar_vec.extend(self.E_var.to_sponge_field_elements()?);

        // Extend the vector with the other FpVar fields
        fpvar_vec.extend(self.x_var.clone());
        fpvar_vec.extend(self.y_var.clone());
        fpvar_vec.push(self.z_var.clone());

        // Return the concatenated vector
        Ok(fpvar_vec)
    }
}


#[cfg(test)]
pub mod tests {
    use std::fmt::Debug;
    use std::iter::zip;

    use ark_crypto_primitives::sponge::constraints::AbsorbGadget;
    use ark_ec::AffineRepr;
    use ark_ec::pairing::Pairing;
    use ark_r1cs_std::alloc::{AllocationMode, AllocVar};
    use ark_r1cs_std::R1CSVar;
    use ark_relations::r1cs::ConstraintSystem;
    use ark_std::UniformRand;
    use rand::thread_rng;

    use crate::accumulation::accumulator::AccInstance;
    use crate::accumulation_circuit::instance_circuit::AccumulatorInstanceVar;
    use crate::constant_for_curves::{E, ScalarField};

    fn get_random_acc_instance() -> AccInstance<E> {
        AccInstance::<E> {
            C: <E as Pairing>::G1Affine::rand(&mut thread_rng()),
            T: <E as Pairing>::G1Affine::rand(&mut thread_rng()),
            E: <E as Pairing>::G1Affine::rand(&mut thread_rng()),
            x: vec![ScalarField::rand(&mut thread_rng()),
                    ScalarField::rand(&mut thread_rng()),
            ],
            y: vec![ScalarField::rand(&mut thread_rng()),
                    ScalarField::rand(&mut thread_rng()),
                    ScalarField::rand(&mut thread_rng()),
                    ScalarField::rand(&mut thread_rng()),
            ],
            z: ScalarField::rand(&mut thread_rng()),
        }
    }
    #[test]
    fn initialisation_test() {
        // build an instance of AccInstanceCircuit
        let instance = get_random_acc_instance();

        // a constraint system
        let cs = ConstraintSystem::<ScalarField>::new_ref();

        // make a circuit_var
        let circuit_var = AccumulatorInstanceVar::new_variable(
            cs,
            || Ok(instance.clone()),
            AllocationMode::Constant,
        ).unwrap();

        // get its value and assert its equal to the original instance
        let c = circuit_var.value().unwrap();

        assert_eq!(c, instance, "the value function doesn't work");
    }

    #[test]
    fn absorb_test() {
        let instance = get_random_acc_instance();

        // a constraint system
        let cs = ConstraintSystem::<ScalarField>::new_ref();

        // make a circuit_var
        let instance_var = AccumulatorInstanceVar::new_variable(
            cs.clone(),
            || Ok(instance.clone()),
            AllocationMode::Witness,
        ).unwrap();

        println!("{}", cs.num_constraints());

        let sponge = instance.to_sponge_field_elements();
        let sponge_var = instance_var.to_sponge_field_elements().unwrap();

        for (x, x_var) in zip(instance.to_sponge_field_elements(), instance_var.to_sponge_field_elements().unwrap()) {
            assert_eq!(x, x_var.value().unwrap());
        }
    }
}

