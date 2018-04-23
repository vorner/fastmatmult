use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::ops::{Index, IndexMut};
use std::path::Path;

use bincode;
use failure::Error;
use rand::{self, Rng};

use super::Element;

pub struct Rows<'a> {
    matrix: &'a Matrix,
    pos: usize,
}

impl<'a> Iterator for Rows<'a> {
    type Item = &'a [Element];
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.matrix.height {
            None
        } else {
            let start = self.matrix.width * self.pos;
            self.pos += 1;
            Some(&self.matrix.content[start .. start + self.matrix.width])
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matrix {
    width: usize,
    height: usize,
    content: Vec<Element>,
}

impl Matrix {
    fn validate(&self) {
        assert_eq!(self.content.len(), self.width * self.height);
    }
    pub fn sized(w: usize, h: usize) -> Self {
        Self {
            width: w,
            height: h,
            content: vec![Element::default(); w * h],
        }
    }
    pub fn random(w: usize, h: usize) -> Self {
        let mut result = Self::sized(w, h);
        let mut rng = rand::thread_rng();
        for x in 0..w {
            for y in 0..h {
                result[(x, y)] = rng.gen_range(0., 10.);
            }
        }
        result
    }
    pub fn rows(&self) -> Rows {
        Rows {
            matrix: self,
            pos: 0,
        }
    }
    pub fn load(file: &Path) -> Result<Self, Error> {
        let f = File::open(file)?;
        Ok(bincode::deserialize_from(BufReader::new(f))?)
    }
    pub fn store(&self, file: &Path) -> Result<(), Error> {
        let f = File::create(file)?;
        bincode::serialize_into(BufWriter::new(f), self)?;
        Ok(())
    }
    pub fn height(&self) -> usize { self.height }
    pub fn width(&self) -> usize { self.width }
    pub(crate) fn slice(&self) -> Slice {
        Slice {
            width: self.width,
            height: self.height,
            content: &self.content,
        }
    }
    pub(crate) fn slice_mut(&mut self) -> SliceMut {
        SliceMut {
            width: self.width,
            height: self.height,
            content: &mut self.content,
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

pub(crate) struct Slice<'a> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) content: &'a [Element],
}

impl<'a> Index<(usize, usize)> for Slice<'a> {
    type Output = Element;
    fn index(&self, index: (usize, usize)) -> &Element {
        &self.content[index.0 + self.width * index.1]
    }
}

pub(crate) struct SliceMut<'a> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) content: &'a mut [Element],
}

impl<'a> Index<(usize, usize)> for SliceMut<'a> {
    type Output = Element;
    fn index(&self, index: (usize, usize)) -> &Element {
        &self.content[index.0 + self.width * index.1]
    }
}

impl<'a> IndexMut<(usize, usize)> for SliceMut<'a> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Element {
        &mut self.content[index.0 + self.width * index.1]
    }
}

pub(crate) fn multiply_add(into: &mut SliceMut, a: &Slice, b: &Slice) {
    assert_eq!(a.width, b.height);

    let w = into.width;
    let h = into.height;
    let l = a.width;

    for x in 0..w {
        for y in 0..h {
            for p in 0..l {
                into[(x, y)] += a[(p, y)] * b[(x, p)];
            }
        }
    }
}

pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut r = Matrix::sized(b.width, a.height);

    multiply_add(&mut r.slice_mut(), &a.slice(), &b.slice());

    // These serve two purposes:
    // * Sanity check the matrix implementations.
    // * Allow the optimiser to remove the range checks from the below indexing.
    a.validate();
    b.validate();
    r.validate();

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Matrix {
        pub(crate) fn identity(size: usize) -> Self {
            let mut r = Self::sized(size, size);
            for i in 0..size {
                r[(i, i)] = 1.0;
            }
            r
        }
    }

    #[test]
    fn add_mult() {
        let mut result = Matrix::sized(2, 2);
        let id = Matrix::identity(2);
        multiply_add(&mut result.slice_mut(), &id.slice(), &id.slice());
        assert_eq!(result, id);
        multiply_add(&mut result.slice_mut(), &id.slice(), &id.slice());
        let double = Matrix {
            width: 2,
            height: 2,
            content: vec![
                2., 0.,
                0., 2.0,
            ],
        };
        assert_eq!(result, double);
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
