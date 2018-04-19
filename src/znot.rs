use typenum::Unsigned;

use super::Element;
use super::simple::{self, Matrix as Simple, Slice, SliceMut};

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

pub fn multiply<Frag: Unsigned + Default>(a: &Matrix<Frag>, b: &Matrix<Frag>) -> Matrix<Frag> {
    assert_eq!(a.size, b.size);
    let mut result = Matrix {
        _frag: Frag::default(),
        size: a.size,
        content: vec![0.; a.size * a.size],
    };

    fn mult_add(r: &mut [Element], a: &[Element], b: &[Element], size: usize, frag: usize) {
        if size == frag {
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
        } else {
            let size = size / 2;
            let block = size * size;
            let a00 = &a[0 .. block];
            let a01 = &a[block .. 2 * block];
            let a10 = &a[2 * block .. 3 * block];
            let a11 = &a[3 * block .. 4 * block];
            let b00 = &b[0 .. block];
            let b01 = &b[block .. 2 * block];
            let b10 = &b[2 * block .. 3 * block];
            let b11 = &b[3 * block .. 4 * block];

            mult_add(&mut r[0 .. block], a00, b00, size, frag);
            mult_add(&mut r[0 .. block], a01, b10, size, frag);

            mult_add(&mut r[block .. 2 * block], a00, b01, size, frag);
            mult_add(&mut r[block .. 2 * block], a01, b11, size, frag);

            mult_add(&mut r[2 * block .. 3 * block], a10, b00, size, frag);
            mult_add(&mut r[2 * block .. 3 * block], a11, b10, size, frag);

            mult_add(&mut r[3 * block .. 4 * block], a10, b01, size, frag);
            mult_add(&mut r[3 * block .. 4 * block], a11, b11, size, frag);
        }
    }
    mult_add(&mut result.content, &a.content, &b.content, a.size, Frag::USIZE);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    use typenum::{U1, U2, U7, U16};

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
            let r_z = multiply(&a_z, &b_z);
            let result = Simple::from(&r_z);
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
