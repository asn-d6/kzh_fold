use ark_ec::short_weierstrass::Projective;

pub mod r1cs;
mod ova;

pub(crate) type R1CSShape<G> = r1cs::R1CSShape<Projective<G>>;
pub(crate) type R1CSInstance<G, C> = r1cs::R1CSInstance<Projective<G>, C>;
pub(crate) type R1CSWitness<G> = r1cs::R1CSWitness<Projective<G>>;
pub(crate) type RelaxedR1CSInstance<G, C> = r1cs::RelaxedR1CSInstance<Projective<G>, C>;
pub(crate) type RelaxedR1CSWitness<G> = r1cs::RelaxedR1CSWitness<Projective<G>>;
