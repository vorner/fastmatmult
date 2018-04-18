use typenum::Unsigned;

use super::Element;
use super::simple::Matrix as Simple;

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

#[cfg(test)]
mod tests {
    use super::*;

    use typenum::{U1, U2, U7, U16};

    fn test_tab<Frag: Unsigned + Default>() {
        for shift in 0..7 {
            let matrix = Simple::random(16 << shift, 16 << shift);
            let there = Matrix::<U16>::from(&matrix);
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
}
