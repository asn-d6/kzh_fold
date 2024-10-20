use ark_ec::short_weierstrass::Projective;

pub mod r1cs;
pub mod ova;

pub(crate) type R1CSShape<G> = r1cs::R1CSShape<Projective<G>>;
pub(crate) type R1CSInstance<G, C> = r1cs::R1CSInstance<Projective<G>, C>;
pub(crate) type R1CSWitness<G> = r1cs::R1CSWitness<Projective<G>>;
pub(crate) type RelaxedR1CSInstance<G, C> = r1cs::RelaxedR1CSInstance<Projective<G>, C>;
pub(crate) type RelaxedR1CSWitness<G> = r1cs::RelaxedR1CSWitness<Projective<G>>;
pub type OvaInstance<G, C> = ova::OvaInstance<Projective<G>, C>;
pub type OvaWitness<G> = ova::OvaWitness<Projective<G>>;
pub type RelaxedOvaInstance<G, C> = ova::RelaxedOvaInstance<Projective<G>, C>;
pub type RelaxedOvaWitness<G> = ova::RelaxedOvaWitness<Projective<G>>;


