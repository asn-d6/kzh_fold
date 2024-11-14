Code Structure
--------------

```
src/
├── accumulation: KZH accumulation implementation (out of circuit)
├── accumulation_circuit: Circuit implementation of the KZH accumulation verifier
├── augmented_circuit: Augmented circuit implementation for KZH-based folding it includes KZH acc verifier circuit, Spartan partial verifier and matrix evaluation accumulator verifier
├── halo_infinite: Halo Infinite Private Aggregation
├── hash: Poseidon implementation (in circuit and out of circuit)
├── nexus_spartan: Spartan implementation by Nexus (plus our own modifications as indicated below)
│   ├── matrix_evaluation_accumulator: Implementation of ABC matrices accumulation
│   ├── partial_verifier: Spartan partial verifier implementation inside the circuit
│   ├── polycommitment: KZH wrapper used for Spartan
│   ├── sumcheck_circuit: Sumcheck verification circuit
├── nova: Nova implementation for benchmarking purposes
├── pcs: Implementation of the KZH and KZH_3 PCS
├── signature_aggregation: Implementation of the signature aggregation protocol
```
