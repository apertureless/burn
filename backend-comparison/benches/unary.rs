use backend_comparison::persistence::save;
use burn::tensor::{backend::Backend, Distribution, Shape, Tensor};
use burn_common::benchmark::{run_benchmark, Benchmark};
use derive_new::new;

#[derive(new)]
struct UnaryBenchmark<B: Backend, const D: usize> {
    shape: Shape<D>,
    num_repeats: usize,
    device: B::Device,
}

impl<B: Backend, const D: usize> Benchmark for UnaryBenchmark<B, D> {
    type Args = Tensor<B, D>;

    fn name(&self) -> String {
        "unary".into()
    }

    fn shapes(&self) -> Vec<Vec<usize>> {
        vec!(self.shape.dims.into())
    }

    fn num_repeats(&self) -> usize {
        self.num_repeats
    }

    fn execute(&self, args: Self::Args) {
        for _ in 0..self.num_repeats() {
            // Choice of tanh is arbitrary
            B::tanh(args.clone().into_primitive());
        }
    }

    fn prepare(&self) -> Self::Args {
        Tensor::random(self.shape.clone(), Distribution::Default, &self.device)
    }

    fn sync(&self) {
        B::sync(&self.device)
    }
}

#[allow(dead_code)]
fn bench<B: Backend>(device: &B::Device) {
    const D: usize = 3;
    let shape: Shape<D> = [32, 512, 1024].into();
    let num_repeats = 10;

    let benchmark = UnaryBenchmark::<B, D>::new(shape, num_repeats, device.clone());

    save::<B>(vec![run_benchmark(benchmark)], device).unwrap();
}

fn main() {
    backend_comparison::bench_on_backend!();
}
