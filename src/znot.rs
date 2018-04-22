use std::ops::{AddAssign, Mul};

use typenum::Unsigned;
use rayon::prelude::*;

use super::Element;
use super::simple::{self, Matrix as Simple, Slice, SliceMut};

pub trait Frag: Default {
    type Elem: AddAssign + Mul<Output = Self::Elem> + Copy + Default;
    const SIZE: usize;
    const BATCH: usize;
    fn load(data: &[Element], offset: usize) -> Self::Elem;
    fn store(elem: Self::Elem, data: &mut [Element], offset: usize);
}

impl<U: Unsigned + Default> Frag for U {
    type Elem = Element;
    const SIZE: usize = U::USIZE;
    const BATCH: usize = 1;
    fn load(data: &[Element], offset: usize) -> Element {
        data[offset]
    }
    fn store(elem: Element, data: &mut [Element], offset: usize) {
        data[offset] = elem;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix<F: Frag> {
    _frag: F,
    size: usize,
    content: Vec<F::Elem>,
}

impl<'a, F: Frag> From<&'a Simple> for Matrix<F> {
    fn from(matrix: &'a Simple) -> Self {
        fn convert<F: Frag>(
            matrix: &Simple,
            content: &mut Vec<F::Elem>,
            x: usize,
            y: usize,
            s: usize,
        ) {
            if s == F::SIZE {
                for j in 0..F::SIZE {
                    for i in 0..F::SIZE / F::BATCH {
                        let xpos = x + i * F::BATCH;
                        let ypos = y + j;
                        let pos = xpos + ypos * matrix.width();
                        content.push(F::load(&matrix.content, pos));
                    }
                }
            } else {
                let s = s / 2;
                convert::<F>(matrix, content, x, y, s);
                convert::<F>(matrix, content, x + s, y, s);
                convert::<F>(matrix, content, x, y + s, s);
                convert::<F>(matrix, content, x + s, y + s, s);
            }
        }

        let size = matrix.width();

        assert_eq!(F::SIZE % F::BATCH, 0, "Unbatchable fragment size");
        assert_eq!(matrix.width(), matrix.height(), "We support only square matrices");
        assert_eq!(size % F::SIZE, 0, "Matrix size must be multiple of {}", F::SIZE);
        assert_eq!((size / F::SIZE).count_ones(), 1, "Matrix size must be power of 2");

        let mut content = Vec::with_capacity(size * size / F::BATCH);
        convert::<F>(matrix, &mut content, 0, 0, size);
        Self {
            _frag: F::default(),
            size,
            content,
        }
    }
}

impl<'a, F: Frag> From<&'a Matrix<F>> for Simple {
    fn from(matrix: &'a Matrix<F>) -> Self {
        fn convert<F: Frag>(
            matrix: &Matrix<F>,
            result: &mut Simple,
            x: usize,
            y: usize,
            s: usize,
            pos: &mut usize,
        ) {
            if s == F::SIZE {
                for j in 0..F::SIZE {
                    for i in 0..F::SIZE / F::BATCH {
                        let xpos = x + i * F::BATCH;
                        let ypos = y + j;
                        let dpos = xpos + ypos * matrix.size;
                        F::store(matrix.content[*pos], &mut result.content, dpos);
                        *pos += 1;
                    }
                }
            } else {
                let s = s / 2;
                convert(matrix, result, x, y, s, pos);
                convert(matrix, result, x + s, y, s, pos);
                convert(matrix, result, x, y + s, s, pos);
                convert(matrix, result, x + s, y + s, s, pos);
            }
        }
        let mut result = Simple::sized(matrix.size, matrix.size);
        convert(matrix, &mut result, 0, 0, matrix.size, &mut 0);
        result
    }
}

pub trait Distribute {
    fn run<I: Send, F: Fn(&mut I) + Send + Sync>(size: usize, tasks: &mut [I], f: F);
}

pub struct DontDistribute;

impl Distribute for DontDistribute {
    fn run<I: Send, F: Fn(&mut I) + Send + Sync>(_: usize, tasks: &mut [I], f: F) {
        for task in tasks {
            f(task);
        }
    }
}

pub struct RayonDistribute<Limit: Unsigned>(pub Limit);

impl<Limit: Unsigned> Distribute for RayonDistribute<Limit> {
    fn run<I: Send, F: Fn(&mut I) + Send + Sync>(size: usize, tasks: &mut [I], f: F) {
        if size >= Limit::USIZE {
            tasks
                .into_par_iter()
                .for_each(f);
        } else {
            DontDistribute::run(size, tasks, f);
        }
    }
}

pub fn multiply<F, Dist>(a: &Matrix<F>, b: &Matrix<F>) -> Matrix<F>
where
    F: Frag,
    Dist: Distribute,
{
    assert_eq!(a.size, b.size);
    let mut result = Matrix {
        _frag: F::default(),
        size: a.size,
        content: vec![F::Elem::default(); a.size * a.size / F::BATCH],
    };

    fn mult_add<F: Flag, Dist: Distribute>(
        r: &mut [F::Elem],
        a: &[F::Elem],
        b: &[F::Elem],
        size: usize,
    ) {
        if size == F::SIZE {
            simple::multiply_add(
                &mut SliceMut {
                    width: size / F::BATCH,
                    height: size,
                    content: r,
                },
                &Slice {
                    width: size / F::,
                    height: size,
                    content: a,
                },
                &Slice {
                    width: size,
                    height: size,
                    content: b,
                },
            );
        } else {
            let s = size / 2;
            let block = s * s;
            let a00 = &a[0 .. block];
            let a01 = &a[block .. 2 * block];
            let a10 = &a[2 * block .. 3 * block];
            let a11 = &a[3 * block .. 4 * block];
            let b00 = &b[0 .. block];
            let b01 = &b[block .. 2 * block];
            let b10 = &b[2 * block .. 3 * block];
            let b11 = &b[3 * block .. 4 * block];

            let (r00, rest) = r.split_at_mut(block);
            let (r01, rest) = rest.split_at_mut(block);
            let (r10, r11) = rest.split_at_mut(block);
            let mut tasks = [
                (r00, a00, b00, a01, b10),
                (r01, a00, b01, a01, b11),
                (r10, a10, b00, a11, b10),
                (r11, a10, b01, a11, b11),
            ];
            Dist::run(size, &mut tasks, |&mut (ref mut r, ref a1, ref b1, ref a2, ref b2)| {
                mult_add::<Dist>(r, a1, b1, s, frag);
                mult_add::<Dist>(r, a2, b2, s, frag);
            });
        }
    }
    mult_add::<Dist>(&mut result.content, &a.content, &b.content, a.size, F::SIZE);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    use typenum::{U1, U2, U7, U16, U32};

    fn test_tab<Frag: Unsigned + Default>() {
        for shift in 0..7 {
            let matrix = Simple::random(Frag::USIZE * 1 << shift, Frag::USIZE * 1 << shift);
            let there = Matrix::<Frag>::from(&matrix);
            let back = Simple::from(&there);
            assert_eq!(matrix, back);
        }
    }

    #[test]
    fn there_and_back_16() {
        test_tab::<U16>();
    }

    #[test]
    fn there_and_back_1() {
        test_tab::<U1>();
    }

    #[test]
    fn there_and_back_2() {
        test_tab::<U2>();
    }

    #[test]
    fn there_and_back_7() {
        test_tab::<U7>();
    }

    #[test]
    fn no_frag() {
        // Matrix 2*2 stays the same
        let ar = vec![1., 2., 3., 4.];
        let mut matrix = Simple::sized(2, 2);
        matrix[(0, 0)] = 1.;
        matrix[(1, 0)] = 2.;
        matrix[(0, 1)] = 3.;
        matrix[(1, 1)] = 4.;
        let conv = Matrix::<U1>::from(&matrix);
        let exp = Matrix {
            _frag: U1::default(),
            size: 2,
            content: ar.clone(),
        };
        assert_eq!(exp, conv);
        let back = Simple::from(&conv);
        assert_eq!(matrix, back);
    }

    fn test_multi<Frag: Unsigned + Default>() {
        for shift in 0..5 {
            let a = Simple::random(Frag::USIZE * 1 << shift, Frag::USIZE * 1 << shift);
            let b = Simple::random(Frag::USIZE * 1 << shift, Frag::USIZE * 1 << shift);
            let expected = simple::multiply(&a, &b);
            let a_z = Matrix::<Frag>::from(&a);
            let b_z = Matrix::<Frag>::from(&b);
            let r_z = multiply::<_, DontDistribute>(&a_z, &b_z);
            let result = Simple::from(&r_z);
            assert_eq!(expected, result);
            let ra_z = multiply::<_, RayonDistribute<U32>>(&a_z, &b_z);
            let result = Simple::from(&ra_z);
            assert_eq!(expected, result);
        }
    }

    #[test]
    fn test_multi_1() {
        test_multi::<U1>();
    }

    #[test]
    fn test_multi_2() {
        test_multi::<U2>();
    }

    #[test]
    fn test_multi_7() {
        test_multi::<U7>();
    }

    #[test]
    fn test_multi_16() {
        test_multi::<U16>();
    }
}
