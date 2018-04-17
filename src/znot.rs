use super::Element;
use super::simple::Matrix as Simple;

pub const FRAGMENT_SIZE: usize = 16;

pub struct Fragment([Element; FRAGMENT_SIZE * FRAGMENT_SIZE]);

impl Default for Fragment {
    fn default() -> Self {
        Fragment([0.; FRAGMENT_SIZE * FRAGMENT_SIZE])
    }
}

pub struct Matrix {
    size: usize,
    content: Vec<Fragment>,
}

impl<'a> From<&'a Simple> for Matrix {
    fn from(matrix: &'a Simple) -> Self {
        fn convert(matrix: &Simple, content: &mut Vec<Fragment>, x: usize, y: usize, s: usize) {
            if s == FRAGMENT_SIZE {
                let mut f = Fragment::default();
                for i in 0..FRAGMENT_SIZE {
                    for j in 0..FRAGMENT_SIZE {
                        f.0[j * FRAGMENT_SIZE + i] = matrix[(i + x, j + y)];
                    }
                }
                content.push(f);
            } else {
                let s = s / 2;
                convert(matrix, content, x, y, s);
                convert(matrix, content, x + s, y, s);
                convert(matrix, content, x, y + s, s);
                convert(matrix, content, x + s, y + s, s);
            }
        }

        let size = matrix.width();
        let fragment_cnt = size * size / FRAGMENT_SIZE / FRAGMENT_SIZE;

        assert_eq!(matrix.width(), matrix.height(), "We support only square matrices");
        assert!(size % FRAGMENT_SIZE == 0, "Matrix size must be multiple of {}", FRAGMENT_SIZE);
        assert_eq!(size.count_ones(), 1, "Matrix size must be power of 2");

        let mut content = Vec::with_capacity(fragment_cnt);
        convert(matrix, &mut content, 0, 0, size);
        Self {
            size,
            content,
        }
    }
}

impl<'a> From<&'a Matrix> for Simple {
    fn from(matrix: &'a Matrix) -> Self {
        fn convert(
            matrix: &Matrix,
            result: &mut Simple,
            x: usize,
            y: usize,
            s: usize,
            pos: &mut usize
        ) {
            if s == FRAGMENT_SIZE {
                for i in 0..FRAGMENT_SIZE {
                    for j in 0..FRAGMENT_SIZE {
                        result[(i + x, j + y)] = matrix.content[*pos].0[j * FRAGMENT_SIZE + i];
                    }
                }
                *pos += 1;
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

    #[test]
    fn there_and_back() {
        for shift in 0..7 {
            let matrix = Simple::random(FRAGMENT_SIZE << shift, FRAGMENT_SIZE << shift);
            let there = Matrix::from(&matrix);
            let back = Simple::from(&there);
            assert_eq!(matrix, back);
        }
    }
}
