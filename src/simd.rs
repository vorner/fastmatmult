use std::iter;

use faster::*;
use smallvec::SmallVec;

use super::simple::{Matrix, Slice, SliceMut};

pub(crate) fn multiply_add(into: &mut SliceMut, a: &Slice, b: &Slice) {
    assert_eq!(a.width, b.height);
    assert_eq!(a.height, into.height);
    assert_eq!(b.width, into.width);

    let h = into.height;
    let l = a.width;

    let pads = iter::repeat(f32s(0.))
        .take(b.width)
        .collect::<SmallVec<[_; 512]>>();
    let columns = b.content
        .simd_iter(f32s(0.));
    let columns = columns
        .stride_into::<SmallVec<[_; 512]>>(b.width, &pads);
    let mut column_data = iter::repeat(0.0)
        .take(b.height)
        .collect::<SmallVec<[_; 512]>>();

    for (x, mut column) in columns.into_iter().enumerate() {
        column.scalar_fill(&mut column_data);
        for y in 0..h {
            let row = &a.content[y * l .. (y + 1) * l];
            into[(x, y)] += (row.simd_iter(f32s(0.)), column_data.simd_iter(f32s(0.))).zip()
                .simd_reduce(f32s(0.0), |acc, (a, b)| acc + a * b)
                .sum();
        }
    }
}

pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut result = Matrix::sized(b.width(), a.height());

    multiply_add(&mut result.slice_mut(), &a.slice(), &b.slice());

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    use ::simple::{self, Matrix};

    /*
     * By using SIMD vectors to sum many at once, we reorder the additions on floats. It so happens
     * this changes the result somewhat, so we put a margin there.
     */
    fn approx_eq(mut a: Matrix, mut b: Matrix) {
        for val in a.slice_mut().content {
            *val = (*val / 20.0).round();
        }
        for val in b.slice_mut().content {
            *val = (*val / 20.0).round();
        }

        assert_eq!(a, b);
    }

    #[test]
    fn test_multi() {
        for shift in 0..7 {
            let s = 1 << shift;
            let a = Matrix::random(s, s);
            let b = Matrix::random(s, s);
            let expected = simple::multiply(&a, &b);
            let result = multiply(&a, &b);
            approx_eq(expected, result);
        }
    }

    #[test]
    fn id() {
        for size in 1..4 {
            let id = Matrix::identity(size);
            let result = multiply(&id, &id);
            approx_eq(result, id);
        }
    }

    #[test]
    fn rect() {
        let a = Matrix::random(2, 3);
        let b = Matrix::random(3, 2);
        let result = multiply(&a, &b);
        let expected = simple::multiply(&a, &b);
        approx_eq(expected, result);
    }
}
