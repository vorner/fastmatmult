use faster::prelude::*;
use rayon::prelude::*;
use typenum::Unsigned;

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

macro_rules! quads {
    ($slice: expr) => {{
        let len = $slice.len() / 4;
        let mut iter = $slice.chunks(len);
        tuplify!(4, iter.next().unwrap())
    }};
    (mut $slice: expr) => {{
        let len = $slice.len() / 4;
        let mut iter = $slice.chunks_mut(len);
        tuplify!(4, iter.next().unwrap())
    }};
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
        frag: usize,
    ) {
        if size == frag {
            Mult::multiply_add(r, a, b, size);
        } else {
            let s = size / 2;
            let (a11, a12, a21, a22) = quads!(a);
            let (b11, b12, b21, b22) = quads!(b);
            let (r11, r12, r21, r22) = quads!(mut r);

            let mut tasks = [
                (r11, a11, b11, a12, b21),
                (r12, a11, b12, a12, b22),
                (r21, a21, b11, a22, b21),
                (r22, a21, b12, a22, b22),
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

macro_rules! op {
    ($res: expr => $first: ident $($op: tt $next: ident)*) => {{
        ($first.simd_iter(f32s(0.)), $($next.simd_iter(f32s(0.)),)*).zip()
            .simd_map(|($first, $($next,)*)| $first $($op $next)*)
            .scalar_fill($res);
    }};
    ($buf: expr, $first: ident $($op: tt $next: ident)*) => {{
        let res: &mut [_] = $buf.next().unwrap();
        op!(res => $first $($op $next)*);
        // Get rid of mut
        &*res
    }};
}

pub fn strassen<Frag, Dist, Mult>(a: &Matrix<Frag>, b: &Matrix<Frag>) -> Matrix<Frag>
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

    fn step<Dist: Distribute, Mult: FragMultiplyAdd>(
        r: &mut [Element],
        a: &[Element],
        b: &[Element],
        size: usize,
        frag: usize,
    ) {
        if size == frag {
            Mult::multiply_add(r, a, b, size);
        } else {
            let s = size / 2;
            let block = s * s;
            let (a11, a12, a21, a22) = quads!(a);
            let (b11, b12, b21, b22) = quads!(b);
            let (r11, r12, r21, r22) = quads!(mut r);

            // We need some auxiliary space (for 17 matrices ‒ or can we optimise? Can we reuse the
            // space of the results?). Allocate it in just one chunk and split it up.
            let mut buffer = vec![0.; 17 * block];
            let mut bc = buffer.chunks_mut(block);

            // Prepare for the smaller multiplications. These are summed/subtracted with SIMD and
            // we don't have to care about the element orders, since both matrices have them the
            // same.
            let m1l = op!(bc, a11 + a22);
            let m1r = op!(bc, b11 + b22);
            let m2l = op!(bc, a21 + a22);
            let m3r = op!(bc, b12 - b22);
            let m4r = op!(bc, b21 - b11);
            let m5l = op!(bc, a11 + a12);
            let m6l = op!(bc, a21 - a11);
            let m6r = op!(bc, b11 + b12);
            let m7l = op!(bc, a21 - a11);
            let m7r = op!(bc, b21 + b22);

            // Run the sub-multiplications, possibly across multiple threads
            let (mut m1, mut m2, mut m3, mut m4, mut m5, mut m6, mut m7) =
                tuplify!(7, bc.next().unwrap());
            let mut tasks = [
                (&mut m1, m1l, m1r),
                (&mut m2, m2l, b11),
                (&mut m3, a11, m3r),
                (&mut m4, a22, m4r),
                (&mut m5, m5l, b22),
                (&mut m6, m6l, m6r),
                (&mut m7, m7l, m7r),
            ];
            Dist::run(size, &mut tasks, |&mut (ref mut r, ref a, ref b)| {
                step::<Dist, Mult>(r, a, b, s, frag);
            });

            // Consolidate the results
            op!(r11 => m1 + m4 - m5 + m7);
            op!(r12 => m3 + m5);
            op!(r21 => m2 + m4);
            op!(r22 => m1 - m2 + m3 + m6);
        }
    }
    step::<Dist, Mult>(&mut result.content, &a.content, &b.content, a.size, Frag::USIZE);

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
            approx_eq(expected.clone(), result);
            let rs_z = strassen::<_, RayonDistribute<U32>, Mult>(&a_z, &b_z);
            let result = Simple::from(&rs_z);
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
