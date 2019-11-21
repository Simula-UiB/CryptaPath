//! Provide a set of tools to execute operations on matrices
//! over GF(2). Can be used to find linear dependencies in a system
//! of equations, solve it and transpose matrices.

use std::fmt;
use std::iter;
use std::slice::Iter;
use vob::Vob;

/// `matrix!` is a sugar around Matrix::from_rows().
///
/// It allows for creating easily `Matrix` object from a
/// `Vec` of `Vob`.
#[macro_export]
macro_rules! matrix {
    [$rows:expr] => {
        $crate::algebra::Matrix::from_rows($rows)
    };
}

/// We define a Matrix as a `Vec` of Vector of bits. Each row be will a `Vob`.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct Matrix {
    rows: Vec<Vob>,
}

impl Matrix {
    /// Create an all-zero matrix of size (rows,colums) specified.
    pub fn new(rows: usize, columns: usize) -> Matrix {
        let mut m = Matrix {
            rows: Default::default(),
        };
        for _i in 0..rows {
            m.rows.push(Vob::from_elem(columns, false));
        }
        m
    }
    /// Create a Matrix from a `Vec` of `Vob`.
    ///
    /// Will panic if all the `Vob` in `rows` don't have the same length.
    pub fn from_rows(rows: Vec<Vob>) -> Matrix {
        let row_size = match rows.get(0) {
            Some(row) => row.len(),
            None => return Matrix::new(0, 0),
        };
        for v in rows.iter().skip(1) {
            if v.len() != row_size {
                panic!("Trying to create a matrix with rows of different size")
            }
        }
        Matrix { rows }
    }

    /// Return an iterator over the rows of the Matrix
    #[inline]
    pub fn iter_rows(&self) -> Iter<Vob> {
        self.rows.iter()
    }

    /// Return the number of rows of the matrix
    #[inline]
    pub fn row_size(&self) -> usize {
        self.rows.len()
    }

    /// Return the number of columns of the matrix
    #[inline]
    pub fn column_size(&self) -> usize {
        match self.rows.get(0) {
            Some(x) => x.len(),
            None => 0,
        }
    }
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Matrix :")?;
        for vob in self.rows.iter() {
            write!(f, "[")?;
            for bit in vob.iter() {
                write!(f, "{}", {
                    if bit {
                        1
                    } else {
                        0
                    }
                })?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

/// Create an identity matrix (a matrix where only the [a,a] element are set)
pub fn identity(rows: usize, columns: usize) -> Matrix {
    assert!(rows == columns);
    let mut m = Matrix::new(rows, columns);
    for i in 0..rows {
        m.rows[i].set(i, true);
    }
    m
}

/// Return the transpose of a matrix
pub fn transpose(matrix: &Matrix) -> Matrix {
    let mut trans = Matrix::new(matrix.column_size(), matrix.row_size());
    for (i, row) in matrix.rows.iter().enumerate() {
        for j in 0..matrix.column_size() {
            trans.rows[j].set(i, row[j]);
        }
    }
    trans
}

/// Return the highest set bit with little endianness.
///
/// ex : 01001 will return 4
#[inline]
pub fn get_max_set_bit(vob: &Vob) -> Option<usize> {
    vob.iter_set_bits(..).last()
}

/// Return the matrix of linear dependencies of the linear system represented
/// by `mat`.
///
/// To compute the matrix of linear dependencies :
///
/// -> augment the given matrix with the identity matrix
///
/// -> gauss the matrix and apply the same operations on the identity matrix
///
/// -> return the lower part of the identity containing the dependencies
pub fn extract_linear_dependencies(mut mat: Matrix) -> Matrix {
    let mut id = identity(mat.row_size(), mat.row_size());
    let mut loop_id = 0;
    for i in (0..mat.row_size()).rev() {
        let mut highest_set_bit = get_max_set_bit(&mat.rows[i]);
        let mut max_row = i;
        for j in (0..i).rev() {
            if get_max_set_bit(&mat.rows[j]).is_some()
                && (highest_set_bit.is_none()
                    || get_max_set_bit(&mat.rows[j]).unwrap() > highest_set_bit.unwrap())
            {
                highest_set_bit = get_max_set_bit(&mat.rows[j]);
                max_row = j;
            }
        }
        if let Some(highest_set_bit) = highest_set_bit {
            if max_row < i {
                mat.rows.swap(i, max_row);
                id.rows.swap(i, max_row);
            }
            for j in (0..i).rev() {
                if get_max_set_bit(&mat.rows[j]).is_some()
                    && get_max_set_bit(&mat.rows[j]).unwrap() == highest_set_bit
                {
                    let to_add = mat.rows[i].clone();
                    mat.rows[j].xor(&to_add);
                    let to_add = id.rows[i].clone();
                    id.rows[j].xor(&to_add);
                }
            }
        } else {
            break;
        }
        loop_id = i;
    }
    id.rows.drain(loop_id..id.row_size());
    for i in (0..id.row_size()).rev() {
        let mut highest_set_bit = get_max_set_bit(&id.rows[i]);
        let mut max_row = i;
        for j in (0..i).rev() {
            if get_max_set_bit(&id.rows[j]).is_some()
                && (highest_set_bit.is_none()
                    || get_max_set_bit(&id.rows[j]).unwrap() > highest_set_bit.unwrap())
            {
                highest_set_bit = get_max_set_bit(&id.rows[j]);
                max_row = j;
            }
        }
        if let Some(highest_set_bit) = highest_set_bit {
            if max_row < i {
                id.rows.swap(i, max_row);
            }
            for j in (0..i).rev() {
                if get_max_set_bit(&id.rows[j]).is_some()
                    && get_max_set_bit(&id.rows[j]).unwrap() == highest_set_bit
                {
                    let to_add = id.rows[i].clone();
                    id.rows[j].xor(&to_add);
                }
            }
        } else {
            break;
        }
    }
    for i in 0..id.row_size() {
        let highest_set_bit = get_max_set_bit(&id.rows[i]);
        for j in i + 1..id.row_size() {
            if id.rows[j][highest_set_bit.unwrap()] {
                let to_add = id.rows[i].clone();
                id.rows[j].xor(&to_add);
            }
        }
    }
    id
}

/// Solve a linear system represented by a `Matrix` (left hand side) and a `Vob` (right hand side).
///
/// To solve we augment the lhs with the rhs and use gaussian elimination.
///
/// Once the matrix is reduced the solution will be a `Vec` of `Some(bool)` for every fixed variable,
/// and `None` for every free variable.
pub fn solve_linear_system(mut lhs: Matrix, mut rhs: Vob) -> Vec<Option<bool>> {
    for i in (0..lhs.row_size()).rev() {
        let mut highest_set_bit = get_max_set_bit(&lhs.rows[i]);
        let mut max_row = i;
        for j in (0..i).rev() {
            if get_max_set_bit(&lhs.rows[j]).is_some()
                && (highest_set_bit.is_none()
                    || get_max_set_bit(&lhs.rows[j]).unwrap() > highest_set_bit.unwrap())
            {
                highest_set_bit = get_max_set_bit(&lhs.rows[j]);
                max_row = j;
            }
        }
        if let Some(highest_set_bit) = highest_set_bit {
            if max_row < i {
                lhs.rows.swap(i, max_row);
                let value_max_row = rhs[max_row];
                let value_i = rhs[i];
                rhs.set(i, value_max_row);
                rhs.set(max_row, value_i);
            }
            for j in (0..i).rev() {
                if get_max_set_bit(&lhs.rows[j]).is_some()
                    && get_max_set_bit(&lhs.rows[j]).unwrap() == highest_set_bit
                {
                    let to_add = lhs.rows[i].clone();
                    lhs.rows[j].xor(&to_add);
                    rhs.set(j, rhs[i] ^ rhs[j]);
                }
            }
        } else {
            break;
        }
    }
    for i in 0..lhs.row_size() {
        let highest_set_bit = get_max_set_bit(&lhs.rows[i]);
        for j in i + 1..lhs.row_size() {
            if lhs.rows[j][highest_set_bit.unwrap()] {
                let to_add = lhs.rows[i].clone();
                lhs.rows[j].xor(&to_add);
                rhs.set(j, rhs[i] ^ rhs[j]);
            }
        }
    }
    let mut solutions: Vec<Option<bool>> = iter::repeat(None).take(lhs.column_size()).collect();

    for (index_row, row) in lhs.iter_rows().enumerate() {
        if let Some(b) = row.iter_set_bits(..).next() {
            if b == get_max_set_bit(row).unwrap() {
                solutions[b] = Some(rhs[index_row]);
            }
        }
    }
    solutions
}

#[cfg(test)]
mod test;
