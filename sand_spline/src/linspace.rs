use num_traits::cast::FromPrimitive;
use num_traits::Float;

pub struct Linspace<T: Float + FromPrimitive> {
    increment: T,
    n: usize,
    x: T,
    i: usize
}

impl<T: Float + FromPrimitive> Iterator for Linspace<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.i == self.n {return None;}

        let x = self.x;
        self.x = self.x + self.increment;
        self.i += 1;
        Some(x)
    }
}

pub fn linspace<T: Float + FromPrimitive>(from: T, to: T, n:usize) -> Linspace<T> {
    Linspace{
        increment: (to - from) / T::from_usize((n - 1).max(0)).unwrap(),
        n: n,
        x: from,
        i: 0
    }
}

