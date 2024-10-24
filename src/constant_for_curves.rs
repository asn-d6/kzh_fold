use ark_bn254::g1::Config as BNConfig;
use ark_bn254::g1::G1Affine as g1;
use ark_bn254::g2::G2Affine as g2;
use ark_bn254::{Bn254, Fq, Fr};
use ark_ec::short_weierstrass::Projective;
use ark_grumpkin::GrumpkinConfig;

/// Since we use the cycle of curves (Bn254, Grumpkin) throughout our tests, we define some types here, so later we can easily use them in out tests

pub type E = Bn254;

pub type ScalarField = Fr;

pub type BaseField = Fq;

pub type G1Affine = g1;

pub type G2Affine = g2;

pub type G1 = BNConfig;

pub type G2 = GrumpkinConfig;

pub type G1Projective = Projective<G1>;

#[cfg(test)]
mod test {
    use ark_ec::AffineRepr;
    use ark_std::UniformRand;
    use rand::thread_rng;
    use crate::constant_for_curves::{E, G2Affine};

    #[test]
    pub fn test() {
        let g  = G2Affine::rand(&mut thread_rng());
        println!("g: {}", g.x().unwrap());
    }
}