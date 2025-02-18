# KZH

Implementation of:
- KZH polynomial commitment scheme
- KZH accumulation
- Folding based on KZH
- Signature aggregation based on KZH
- HaloInfinite private aggregation

## Building & Running

This library can be compiled with `cargo build` and requires rust nightly.

You can run the tests using `cargo test --release` and the benchmarks using `cargo bench`.

## Code Structure

```
src/
├── kzh_fold: KZH accumulation implementation (out of circuit)
├── kzh2_augmented_circuit: Augmented circuit implementation for KZH2-based folding it includes KZH acc verifier circuit, Spartan partial verifier and matrix evaluation accumulator verifier
├── kzh3_augmented_circuit: Same as above but for KZH3
├── halo_infinite: Halo Infinite Private Aggregation
├── hash: Poseidon implementation (in circuit and out of circuit)
├── nexus_spartan: Spartan implementation by Nexus (plus our own modifications as indicated below)
│   ├── matrix_evaluation_accumulator: Implementation of ABC matrices accumulation
│   ├── partial_verifier: Spartan partial verifier implementation inside the circuit
│   ├── polycommitment: KZH wrapper used for Spartan
│   ├── sumcheck_circuit: Sumcheck verification circuit
├── nova: Nova implementation for benchmarking purposes
├── kzh: Implementation of the KZH and KZH_3 PCS
├── signature_aggregation: Implementation of the signature aggregation protocol
```

## Acknowledgements

This work would not be possible without the arkworks, Nexus and Spartan projects.
