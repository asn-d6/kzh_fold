#![allow(non_snake_case)]
#![allow(unused_imports)]

use ark_poly::EvaluationDomain;
use ark_std::UniformRand;
use criterion::{Criterion, criterion_group, criterion_main};
use rand::thread_rng;

use sqrtn_pcs::constant_for_curves::{E, ScalarField};
use sqrtn_pcs::pcs::kzh2::{PCSEngine, KZH2SRS};
use sqrtn_pcs::polynomial::multilinear_poly::multilinear_poly::MultilinearPolynomial;

// fn bench_setup(c: &mut Criterion) {
//     let degrees = vec![(4, 4), (8, 8), (16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)];
//     for (degree_x, degree_y) in degrees {
//         let bench_name = format!("setup for degrees n={} * m={} (witness size: {})", degree_x, degree_y, degree_x*degree_y);
//         c.bench_function(&bench_name, |b| {
//             b.iter_custom(|iters| {
//                 let mut total_time = std::time::Duration::new(0, 0);
//                 for _ in 0..iters {
//                     let start = std::time::Instant::now();
//                     let _srs: PolynomialCommitmentSRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());
//                     total_time += start.elapsed();
//                 }
//                 total_time
//             });
//         });
//     }
// }

fn bench_commit(c: &mut Criterion) {
    let degrees = vec![(4, 4), (8, 8), (16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)];
    for (degree_x, degree_y) in degrees {
        let srs: KZH2SRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());
        // random bivariate polynomial
        let polynomial = MultilinearPolynomial::rand(
            srs.get_x_length() + srs.get_y_length(),
            &mut thread_rng(),
        );
        let bench_name = format!("commit for degrees n={} * m={} (witness size: {})", degree_x, degree_y, degree_x*degree_y);
        c.bench_function(&bench_name, |b| {
            b.iter(|| {
                PCSEngine::commit(&srs, &polynomial)
            })
        });
    }
}

fn bench_open(c: &mut Criterion) {
    let degrees = vec![(4, 4), (8, 8), (16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)];
    for (degree_x, degree_y) in degrees {
        let srs: KZH2SRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());
        // random bivariate polynomial
        let polynomial = MultilinearPolynomial::rand(
            srs.get_x_length() + srs.get_y_length(),
            &mut thread_rng(),
        );
        let com = PCSEngine::commit(&srs, &polynomial);

        let bench_name = format!("open for degrees n={} * m={} (witness size: {})", degree_x, degree_y, degree_x*degree_y);
        c.bench_function(&bench_name, |b| {
            let x = vec![
                ScalarField::rand(&mut thread_rng()), ScalarField::rand(&mut thread_rng()),
            ];
            b.iter(|| {
                PCSEngine::open(&polynomial, com.clone(), &x)
            })
        });
    }
}

fn bench_verify(c: &mut Criterion) {
    let degrees = vec![(4, 4), (8, 8), (16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)];
    for (degree_x, degree_y) in degrees {
        let srs: KZH2SRS<E> = PCSEngine::setup(degree_x, degree_y, &mut thread_rng());
        // random bivariate polynomial
        let polynomial = MultilinearPolynomial::rand(
            srs.get_x_length() + srs.get_y_length(),
            &mut thread_rng(),
        );
        let com = PCSEngine::commit(&srs, &polynomial);

        let bench_name = format!("verify for degrees n={} * m={} (witness size: {})", degree_x, degree_y, degree_x*degree_y);
        c.bench_function(&bench_name, |b| {
            // random points and evaluation
            let x = {
                let mut res = Vec::new();
                for _ in 0..srs.get_x_length() {
                    res.push(ScalarField::rand(&mut thread_rng()));
                }
                res
            };
            let y = {
                let mut res = Vec::new();
                for _ in 0..srs.get_y_length() {
                    res.push(ScalarField::rand(&mut thread_rng()));
                }
                res
            };
            let concat = {
                let mut res = vec![];
                res.extend(x.clone());
                res.extend(y.clone());
                res
            };
            let z = polynomial.evaluate(&concat);
            let open = PCSEngine::open(&polynomial, com.clone(), &x);
            b.iter(|| {
                PCSEngine::verify(&srs, &com, &open, &x, &y, &z)
            })
        });
    }
}

fn custom_criterion_config() -> Criterion {
    Criterion::default().sample_size(10)
}

// Benchmark group setup
criterion_group! {
    name = pcs_benches;
    config = custom_criterion_config();
    targets =  bench_commit
}

criterion_main!(pcs_benches);
