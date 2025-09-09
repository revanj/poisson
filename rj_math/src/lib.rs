#![feature(generic_const_exprs)]
#![feature(inherent_associated_types)]
use std::ops;

struct Pred<const B: bool>;
trait True {} trait False {}
impl True for Pred<true> {}
impl False for Pred<false> {}
struct Vector<const N: usize> {
    data: [f32; N]
}

impl<const N: usize> Vector<N> {
    type T = usize;
}

impl<const N: usize> Vector<N> where Pred<{N > 0}>: True {
    pub fn x(&self) -> f32 {
        self.data[0]
    }
    pub fn set_x(&mut self, val: f32) {
        self.data[0] = val;
    }
}
impl<const N: usize> Vector<N> where Pred<{N > 1}>: True {
    pub fn y(&self) -> f32 {
        self.data[1]
    }
    pub fn set_y(&mut self, val: f32) {
        self.data[1] = val;
    }
}
impl<const N: usize> Vector<N> where Pred<{N > 2}>: True {
    pub fn z(&self) -> f32 {
        self.data[2]
    }
    pub fn set_z(&mut self, val: f32) {
        self.data[2] = val;
    }
}

impl<const N: usize> Vector<N> where Pred<{N > 3}>: True {
    pub fn w(&self) -> f32 {
        self.data[3]
    }
    pub fn set_w(&mut self, val: f32) {
        self.data[3] = val;
    }
}

struct Mat<const M: usize, const N: usize> where [(); M * N]: {
    data: [f32; M * N]
}

impl<const M: usize, const N: usize> ops::Index<(usize, usize)> for Mat<M, N> where [(); M * N]: {
    type Output = f32;

    fn index(&self, index: (usize, usize)) -> &f32 {
        &self.data[index.0 * N + index.1]
    }
}

impl<const M: usize, const N: usize> Mat<M, N> where [(); M * N]: {
    pub fn from_slice(init: [f32; M * N]) -> Mat<M, N> { 
        Self {
            data: init
        }
    }
}

impl<const M: usize, const N: usize, const K: usize> ops::Mul<Mat<N, K>> for Mat<M, N> 
    where [(); M * N]:, [(); N * K]:, [(); M * K]: 
{
    type Output = Mat<M, K>;
    
    fn mul(self: Mat<M, N>, rhs: Mat<N, K>) -> Mat<M, K> {
        let mut slice: [f32; M * K] = [0f32; M * K];
        
        for m in 0..M {
            for k in 0..K {
                let mut sum = 0f32;
                for n in 0..N {
                    sum += self[(m, n)] * rhs[(n, k)]
                }
                slice[m * M + k] = sum;
            }
        }
        
        Mat::from_slice(slice)
    }
}