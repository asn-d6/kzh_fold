use ark_crypto_primitives::crh::poseidon::constraints::CRHParametersVar;
/// Poseidon config stolen from sonobe

use ark_crypto_primitives::sponge::{Absorb, CryptographicSponge, poseidon::{PoseidonConfig}};
use ark_crypto_primitives::sponge::constraints::{AbsorbGadget, CryptographicSpongeVar};
use ark_crypto_primitives::sponge::poseidon::{find_poseidon_ark_and_mds, PoseidonSponge};
use ark_crypto_primitives::sponge::poseidon::constraints::PoseidonSpongeVar;
use ark_ff::PrimeField;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::r1cs::ConstraintSystemRef;

pub struct PoseidonHash<F: Absorb + PrimeField> {
    poseidon_params: PoseidonConfig<F>,
    sponge: PoseidonSponge<F>,
}

pub trait PoseidonHashTrait<F: Absorb + PrimeField> {
    fn new() -> Self;

    fn update_sponge<A: Absorb>(&mut self, field_vector: Vec<A>) -> ();

    fn output(&mut self) -> F;
}

impl<F: Absorb + PrimeField> PoseidonHashTrait<F> for PoseidonHash<F> {
    /// This Poseidon configuration generator agrees with Circom's Poseidon(4) in the case of BN254's scalar field
    fn new() -> Self {
        // 120 bit security target as in
        // https://eprint.iacr.org/2019/458.pdf
        // t = rate + 1

        let full_rounds = 8;
        let partial_rounds = 60;
        let alpha = 5;
        let rate = 4;

        let (ark, mds) = find_poseidon_ark_and_mds::<F>(
            F::MODULUS_BIT_SIZE as u64,
            rate,
            full_rounds,
            partial_rounds,
            0,
        );
        let poseidon_params = PoseidonConfig::new(
            full_rounds as usize,
            partial_rounds as usize,
            alpha,
            mds,
            ark,
            rate,
            1,
        );

        Self {
            poseidon_params: poseidon_params.clone(),
            sponge: PoseidonSponge::new(&poseidon_params),
        }
    }

    fn update_sponge<A: Absorb>(&mut self, field_vector: Vec<A>) -> () {
        for field_element in field_vector {
            self.sponge.absorb(&field_element);
        }
    }

    fn output(&mut self) -> F {
        let squeezed_field_element: Vec<F> = self.sponge.squeeze_field_elements(1);
        squeezed_field_element[0]
    }
}

pub struct PoseidonHashVar<F: Absorb + PrimeField> {
    poseidon_params: CRHParametersVar<F>,
    sponge: PoseidonSpongeVar<F>,
}

pub trait PoseidonHashVarTrait<F: Absorb + PrimeField> {
    fn new(cs: ConstraintSystemRef<F>) -> Self;

    fn update_sponge<A: AbsorbGadget<F>>(&mut self, field_vector: Vec<A>) -> ();

    fn output(&mut self) -> FpVar<F>;
}

impl<F: Absorb + PrimeField> PoseidonHashVarTrait<F> for PoseidonHashVar<F> {
    fn new(cs: ConstraintSystemRef<F>) -> Self {
        let hash = PoseidonHash::new();
        // TODO: later don't clone
        let poseidon_params = CRHParametersVar::<F>::new_witness(cs.clone(), || Ok(hash.poseidon_params.clone())).unwrap();
        let sponge = PoseidonSpongeVar::new(cs, &hash.poseidon_params);
        PoseidonHashVar {
            poseidon_params,
            sponge,
        }
    }

    fn update_sponge<A: AbsorbGadget<F>>(&mut self, field_vector: Vec<A>) -> () {
        for field_element in field_vector {
            self.sponge.absorb(&field_element).expect("Error while sponge absorbing");
        }
    }

    fn output(&mut self) -> FpVar<F> {
        let squeezed_field_element: Vec<FpVar<F>> = self.sponge.squeeze_field_elements(1).unwrap();
        squeezed_field_element[0].clone()
    }
}


#[cfg(test)]
mod tests {
    use std::ops::Mul;
    use ark_bn254::{Bn254, Fq, Fr, G1Projective, G2Projective};
    use ark_crypto_primitives::sponge::{Absorb, CryptographicSponge};
    use ark_crypto_primitives::sponge::poseidon::{PoseidonConfig, PoseidonSponge};
    use ark_ec::CurveGroup;
    use ark_ec::short_weierstrass::{SWCurveConfig, Projective};
    use ark_ff::{Field, PrimeField};
    use ark_r1cs_std::alloc::{AllocationMode, AllocVar};
    use ark_r1cs_std::fields::fp::FpVar;
    use ark_r1cs_std::fields::nonnative::NonNativeFieldVar;
    use ark_r1cs_std::{R1CSVar, ToConstraintFieldGadget};
    use ark_relations::r1cs::{ConstraintSystem, ConstraintSystemRef};
    use ark_std::UniformRand;
    use rand::rngs::OsRng;
    use rand::thread_rng;
    use crate::accumulation::poseidon::{PoseidonHash, PoseidonHashTrait, PoseidonHashVar, PoseidonHashVarTrait};

    type FirstCurve = Fr;
    type SecondCurve = Fq;

    #[test]
    fn hash_test() {
        let mut hash_object: PoseidonHash<Fr> = PoseidonHash::new();
        let field_vector: Vec<FirstCurve> = vec![FirstCurve::ONE, FirstCurve::ONE];
        hash_object.update_sponge(field_vector);
        println!("{}", hash_object.output());
        let mut temp = SecondCurve::ONE.to_sponge_field_elements_as_vec::<FirstCurve>();
        println!("{:?}", temp);
        temp.extend(temp.clone());
        hash_object.update_sponge(temp);
        println!("{}", hash_object.output());
    }

    #[test]
    fn hash_var_test() {
        let cs = ConstraintSystem::new_ref();

        let mut hash_object: PoseidonHashVar<Fr> = PoseidonHashVar::new(cs.clone());

        let one_on_first_curve = FpVar::new_variable(
            cs.clone(),
            || Ok(FirstCurve::ONE),
            AllocationMode::Witness,
        ).unwrap();

        let field_vector: Vec<FpVar<FirstCurve>> = vec![one_on_first_curve.clone(), one_on_first_curve.clone()];

        hash_object.update_sponge(field_vector);
        println!("{}", hash_object.output().value().unwrap());

        let one_on_second_curve = NonNativeFieldVar::new_variable(
            cs.clone(),
            || Ok(SecondCurve::ONE),
            AllocationMode::Witness,
        ).unwrap();

        println!("num of constraints before: {}", cs.num_constraints());
        let temp = one_on_second_curve.clone().to_constraint_field().unwrap();
        println!("num of constraints after: {}", cs.num_constraints());

        hash_object.update_sponge(temp);

        println!("{}", hash_object.output().value().unwrap());
    }
}


