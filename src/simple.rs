use std::ops::{Index, IndexMut};

use super::Element;

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    width: usize,
    height: usize,
    content: Vec<Element>,
}

impl Matrix {
    fn validate(&self) {
        assert_eq!(self.content.len(), self.width * self.height);
    }
    fn sized(w: usize, h: usize) -> Self {
        Self {
            width: w,
            height: h,
            content: vec![Element::default(); w * h],
        }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = Element;
    fn index(&self, index: (usize, usize)) -> &Element {
        &self.content[index.0 + self.width * index.1]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Element {
        &mut self.content[index.0 + self.width * index.1]
    }
}

pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    assert_eq!(a.width, b.height);

    let mut r = Matrix::sized(b.width, a.height);

    let w = r.width;
    let h = r.height;
    let l = a.width;

    // These serve two purposes:
    // * Sanity check the matrix implementations.
    // * Allow the optimiser to remove the range checks from the below indexing.
    a.validate();
    b.validate();
    r.validate();

    for x in 0..w {
        for y in 0..h {
            for p in 0..l {
                r[(x, y)] += a[(p, y)] * b[(x, p)];
            }
        }
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Matrix {
        fn identity(size: usize) -> Self {
            let mut r = Self::sized(size, size);
            for i in 0..size {
                r[(i, i)] = 1.0;
            }
            r
        }
    }

    #[test]
    fn square_identity() {
        let id = Matrix::identity(3);
        let other = Matrix {
            width: 3,
            height: 3,
            content: vec![
                2., 3., 4.,
                0., 0., 0.,
                5., 6., 7.,
            ],
        };
        let left_id = multiply(&id, &other);
        assert_eq!(other, left_id);
        let right_id = multiply(&other, &id);
        assert_eq!(other, right_id);
    }

    #[test]
    fn rect_identity() {
        let rect = Matrix {
            width: 2,
            height: 3,
            content: vec![
                1., 2.,
                3., 4.,
                5., 6.,
            ]
        };
        let left_rect = multiply(&Matrix::identity(3), &rect);
        assert_eq!(rect, left_rect);
        let right_rect = multiply(&rect, &Matrix::identity(2));
        assert_eq!(rect, right_rect);
    }

    #[test]
    fn arbitrary() {
        let a = Matrix {
            width: 2,
            height: 3,
            content: vec![
                1., 2.,
                3., 4.,
                5., 6.,
            ],
        };
        let b = Matrix {
            width: 3,
            height: 2,
            content: vec![
                10., 11., 12.,
                13., 14., 15.,
            ],
        };
        let res_a = multiply(&a, &b);
        let exp_a = Matrix {
            width: 3,
            height: 3,
            content: vec![
                36., 39., 42.,
                82., 89., 96.,
                128., 139., 150.,
            ],
        };
        assert_eq!(res_a, exp_a);
        let res_b = multiply(&b, &a);
        let exp_b = Matrix {
            width: 2,
            height: 2,
            content: vec![
                103., 136.,
                130., 172.,
            ],
        };
        assert_eq!(res_b, exp_b);
    }
}
