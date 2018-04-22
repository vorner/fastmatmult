use typenum::Unsigned;
use rayon::prelude::*;

use super::Element;
use super::simple::{self, Matrix as Simple, Slice, SliceMut};
use super::simd;

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix<Frag: Unsigned> {
    _frag: Frag,
    size: usize,
    content: Vec<Element>,
}

impl<'a, Frag: Unsigned + Default> From<&'a Simple> for Matrix<Frag> {
    fn from(matrix: &'a Simple) -> Self {
        fn convert(
            matrix: &Simple,
            content: &mut Vec<Element>,
            x: usize,
            y: usize,
            s: usize,
            frag: usize
        ) {
            if s == frag {
                for j in 0..frag {
                    for i in 0..frag {
                        content.push(matrix[(i + x, j + y)]);
                    }
                }
            } else {
                let s = s / 2;
                convert(matrix, content, x, y, s, frag);
                convert(matrix, content, x + s, y, s, frag);
                convert(matrix, content, x, y + s, s, frag);
                convert(matrix, content, x + s, y + s, s, frag);
            }
        }

        let size = matrix.width();

        assert_eq!(matrix.width(), matrix.height(), "We support only square matrices");
        assert!(size % Frag::USIZE == 0, "Matrix size must be multiple of {}", Frag::USIZE);
        assert_eq!((size / Frag::USIZE).count_ones(), 1, "Matrix size must be power of 2");

        let mut content = Vec::with_capacity(size * size);
        convert(matrix, &mut content, 0, 0, size, Frag::USIZE);
        Self {
            _frag: Frag::default(),
            size,
            content,
        }
    }
}

impl<'a, Frag: Unsigned> From<&'a Matrix<Frag>> for Simple {
    fn from(matrix: &'a Matrix<Frag>) -> Self {
        fn convert<Frag: Unsigned>(
            matrix: &Matrix<Frag>,
            result: &mut Simple,
            x: usize,
            y: usize,
            s: usize,
            pos: &mut usize,
        ) {
            if s == Frag::USIZE {
                for j in 0..Frag::USIZE {
                    for i in 0..Frag::USIZE {
                        result[(i + x, j + y)] = matrix.content[*pos];
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

pub trait FragMultiplyAdd {
    fn multiply_add(r: &mut [Element], a: &[Element], b: &[Element], size: usize);
}

pub struct SimpleMultiplyAdd;

impl FragMultiplyAdd for SimpleMultiplyAdd {
    fn multiply_add(r: &mut [Element], a: &[Element], b: &[Element], size: usize) {
        simple::multiply_add(
            &mut SliceMut {
                width: size,
                height: size,
                content: r,
            },
            &Slice {
                width: size,
                height: size,
                content: a,
            },
            &Slice {
                width: size,
                height: size,
                content: b,
            },
        );
    }
}

pub struct SimdMultiplyAdd;

impl FragMultiplyAdd for SimdMultiplyAdd {
    fn multiply_add(r: &mut [Element], a: &[Element], b: &[Element], size: usize) {
        simd::multiply_add(
            &mut SliceMut {
                width: size,
                height: size,
                content: r,
            },
            &Slice {
                width: size,
                height: size,
                content: a,
            },
            &Slice {
                width: size,
                height: size,
                content: b,
            },
        );
    }
}

pub fn multiply<Frag, Dist, Mult>(a: &Matrix<Frag>, b: &Matrix<Frag>) -> Matrix<Frag>
where
    Frag: Unsigned + Default,
    Dist: Distribute,
    Mult: FragMultiplyAdd,
{
    assert_eq!(a.size, b.size);
    let mut result = Matrix {
        _frag: Frag::default(),
        size: a.size,
        content: vec![0.; a.size * a.size],
    };

    fn mult_add<Dist: Distribute, Mult: FragMultiplyAdd>(
        r: &mut [Element],
        a: &[Element],
        b: &[Element],
        size: usize,
        frag: usize
    ) {
        if size == frag {
            Mult::multiply_add(r, a, b, size);
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
                mult_add::<Dist, Mult>(r, a1, b1, s, frag);
                mult_add::<Dist, Mult>(r, a2, b2, s, frag);
            });
        }
    }
    mult_add::<Dist, Mult>(&mut result.content, &a.content, &b.content, a.size, Frag::USIZE);

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

    /*
     * By using SIMD vectors to sum many at once, we reorder the additions on floats. It so happens
     * this changes the result somewhat, so we put a margin there.
     */
    fn approx_eq(mut a: Simple, mut b: Simple) {
        for val in a.slice_mut().content {
            *val = (*val / 20.0).round();
        }
        for val in b.slice_mut().content {
            *val = (*val / 20.0).round();
        }
    }

    fn test_multi<Frag: Unsigned + Default, Mult: FragMultiplyAdd>() {
        for shift in 0..5 {
            let a = Simple::random(Frag::USIZE * 1 << shift, Frag::USIZE * 1 << shift);
            let b = Simple::random(Frag::USIZE * 1 << shift, Frag::USIZE * 1 << shift);
            let expected = simple::multiply(&a, &b);
            let a_z = Matrix::<Frag>::from(&a);
            let b_z = Matrix::<Frag>::from(&b);
            let r_z = multiply::<_, DontDistribute, Mult>(&a_z, &b_z);
            let result = Simple::from(&r_z);
            approx_eq(expected.clone(), result);
            let ra_z = multiply::<_, RayonDistribute<U32>, Mult>(&a_z, &b_z);
            let result = Simple::from(&ra_z);
            approx_eq(expected, result);
        }
    }

    #[test]
    fn test_multi_1() {
        test_multi::<U1, SimpleMultiplyAdd>();
    }

    #[test]
    fn test_multi_2() {
        test_multi::<U2, SimpleMultiplyAdd>();
    }

    #[test]
    fn test_multi_7() {
        test_multi::<U7, SimpleMultiplyAdd>();
    }

    #[test]
    fn test_multi_16() {
        test_multi::<U16, SimpleMultiplyAdd>();
    }

    #[test]
    fn test_multi_1_simd() {
        test_multi::<U1, SimdMultiplyAdd>();
    }

    #[test]
    fn test_multi_2_simd() {
        test_multi::<U2, SimdMultiplyAdd>();
    }

    #[test]
    fn test_multi_7_simd() {
        test_multi::<U7, SimdMultiplyAdd>();
    }

    #[test]
    fn test_multi_16_simd() {
        test_multi::<U16, SimdMultiplyAdd>();
    }
}
