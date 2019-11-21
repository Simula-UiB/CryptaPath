use crate::crush::algebra::{Matrix, *};

use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::vob::Vob;
use crate::{bit, bit::Bit, bit::*};
use std::cmp;
use std::collections::VecDeque;

pub struct LowMC {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox: Sbox,
    n_sbox: usize,
    init_params: LowMCParams,
}

#[derive(Default)]
struct LowMCParams {
    lin_matrices: Vec<Vec<bool>>,
    round_constants: Vec<Vec<Bit>>,
    key_matrices: Vec<Vec<bool>>,
}

impl LowMC {
    pub fn new(n_rounds: usize, message_length: usize, key_length: usize, n_sbox: usize) -> Self {
        let table = vec![0x00, 0x01, 0x03, 0x06, 0x07, 0x04, 0x05, 0x02];

        let mut lowmc = LowMC {
            n_rounds,
            message_length,
            key_length,
            sbox: Sbox::new(3, 3, table, message_length + key_length),
            n_sbox,
            init_params: Default::default(),
        };
        lowmc.make_init_params();
        lowmc
    }

    fn key_addition(&self, in_bits: Vec<Bit>, round_key: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        assert_eq!(in_bits.len(), round_key.len());
        bit_vector_xoring(in_bits, round_key)
    }

    fn constant_addition(&self, in_bits: Vec<Bit>, round: usize) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        in_bits
            .iter()
            .cloned()
            .zip(
                self.init_params.round_constants[round - 1]
                    .iter()
                    .rev()
                    .cloned(),
            )
            .map(|(in_bit, constant_bit)| in_bit ^ constant_bit)
            .collect()
    }

    fn linear_layer(&self, in_bits: Vec<Bit>, round: usize) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        multiply_with_gf2_matrix(
            &self.init_params.lin_matrices[round - 1],
            self.message_length(),
            self.message_length(),
            &in_bits,
        )
    }

    fn sbox_layer(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..(self.message_length() - self.n_sbox * 3) {
            out_bits.push(in_bits[i].clone())
        }
        let start = self.message_length() - self.n_sbox * 3;
        for i in 0..self.n_sbox {
            out_bits.append(
                &mut self
                    .sbox
                    .apply(in_bits[start + i * 3..start + (i + 1) * 3].to_vec()),
            );
        }
        out_bits
    }

    fn make_round_keys(&self, key: Vec<Bit>) -> Vec<Vec<Bit>> {
        let mut round_keys = Vec::with_capacity(self.n_rounds());
        for r in 0..=self.n_rounds() {
            round_keys.push(multiply_with_gf2_matrix(
                &self.init_params.key_matrices[r],
                self.message_length(),
                self.key_length(),
                &key,
            ))
        }
        round_keys
    }

    fn make_init_params(&mut self) {
        let mut lfsr = init_lfsr();
        let n = self.message_length();
        let k = self.key_length();
        let mut lin_matrices = Vec::with_capacity(self.n_rounds());
        let mut round_constants = Vec::with_capacity(self.n_rounds());
        let mut key_matrices = Vec::with_capacity(self.n_rounds() + 1);
        for _ in 0..self.n_rounds() {
            let mut valid_matrix = false;
            while !valid_matrix {
                let matrix = extract(&mut lfsr, n * n);
                if matrix_rank(&matrix, n, n) == n {
                    lin_matrices.push(matrix);
                    valid_matrix = true;
                }
            }
        }
        for _ in 0..self.n_rounds() {
            round_constants.push(extract(&mut lfsr, n).iter().map(|b| bit!(*b)).collect());
        }
        for _ in 0..=self.n_rounds() {
            let mut valid_matrix = false;

            while !valid_matrix {
                let matrix = extract(&mut lfsr, n * k);
                if matrix_rank(&matrix, n, k) == cmp::min(n, k) {
                    key_matrices.push(matrix);
                    valid_matrix = true;
                }
            }
        }
        self.init_params.lin_matrices = lin_matrices;
        self.init_params.round_constants = round_constants;
        self.init_params.key_matrices = key_matrices;
    }
}

fn init_lfsr() -> VecDeque<bool> {
    let mut lfsr = vec![true; 80].drain(..).collect::<VecDeque<bool>>();
    for _ in 0..160 {
        let tmp = lfsr[62] ^ lfsr[51] ^ lfsr[38] ^ lfsr[23] ^ lfsr[13] ^ lfsr[0];
        lfsr.pop_front();
        lfsr.push_back(tmp);
    }
    lfsr
}

fn extract(lfsr: &mut VecDeque<bool>, t: usize) -> Vec<bool> {
    let mut out = Vec::with_capacity(t);
    for _ in 0..t {
        let mut tmp = false;
        let mut choice = false;
        while !choice {
            tmp = lfsr[62] ^ lfsr[51] ^ lfsr[38] ^ lfsr[23] ^ lfsr[13] ^ lfsr[0];
            lfsr.pop_front();
            lfsr.push_back(tmp);
            choice = tmp;
            tmp = lfsr[62] ^ lfsr[51] ^ lfsr[38] ^ lfsr[23] ^ lfsr[13] ^ lfsr[0];
            lfsr.pop_front();
            lfsr.push_back(tmp);
        }
        out.push(tmp)
    }
    out
}

fn matrix_from_vec_bool(matrix: &[bool], n_rows: usize, n_columns: usize) -> Matrix {
    assert_eq!(matrix.len(), n_rows * n_columns);
    let mut rows = Vec::with_capacity(n_rows);
    for row_index in 0..matrix.len() / n_columns {
        let mut row = Vob::new();
        let row_bits = &matrix
            .iter()
            .skip(row_index * n_columns)
            .take(n_columns)
            .collect::<Vec<&bool>>();
        for bit in row_bits.iter() {
            row.push(**bit);
        }
        rows.push(row);
    }
    matrix![rows]
}

fn matrix_rank(matrix: &[bool], n_rows: usize, n_columns: usize) -> usize {
    let mut m = matrix_from_vec_bool(matrix, n_rows, n_columns);
    let rank = if n_rows > n_columns {
        m = transpose(&m);
        n_columns
    } else {
        n_rows
    };
    let dep = extract_linear_dependencies(m);
    rank - dep.row_size()
}

fn multiply_with_gf2_matrix(
    matrix: &[bool],
    n_rows: usize,
    n_columns: usize,
    in_bits: &[Bit],
) -> Vec<Bit> {
    assert_eq!(matrix.len(), n_columns * n_rows);
    assert_eq!(in_bits.len(), n_columns);
    let mut out_bits = Vec::with_capacity(in_bits.len());
    for row in 0..n_rows {
        let mut r = matrix
            .iter()
            .skip(row * n_columns)
            .take(n_columns)
            .cloned()
            .collect::<Vec<bool>>();
        r = r.iter().cloned().rev().collect();
        let mut tmp = bit!(false);
        for column in 0..n_columns {
            if r[column] {
                tmp ^= in_bits[column].clone()
            }
        }
        out_bits.push(tmp)
    }
    out_bits.iter().cloned().rev().collect()
}

impl Cipher for LowMC {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit> {
        let round_keys = self.make_round_keys(key_bits);
        let mut state = self.key_addition(in_bits, round_keys[0].clone());
        for i in 1..=self.n_rounds() {
            state = self.key_addition(
                self.constant_addition(self.linear_layer(self.sbox_layer(state), i), i),
                round_keys[i].clone(),
            )
        }
        state
    }

    fn message_length(&self) -> usize {
        self.message_length
    }

    fn key_length(&self) -> usize {
        self.key_length
    }

    fn n_rounds(&self) -> usize {
        self.n_rounds
    }

    fn sbox(&self) -> Sbox {
        self.sbox.clone()
    }
}

// from https://github.com/LowMC/lowmc
#[cfg(test)]
#[cfg(not(debug_assertions))]
mod test {
    use crate::bit;
    use crate::targets::{lowmc::LowMC, Cipher};
    #[test]
    fn validate_encrypt() {
        let lowmc = LowMC::new(12, 256, 80, 49);
        let plaintext = bit::bits_from_binary_string("0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001111111111010101");
        let key = bit::bits_from_binary_string(
            "00000000000000000000000000000000000000000000000000000000000000000000000000000001",
        );
        assert_eq!(
        "1010101000101110001111100110101110110100101011000111000100010100101101001100000000101110110100010011101000110111000011000000010001111100100011010111011001000010010111000100110010100100001000011101101011100000001010100101000111110011001011000000011100101100",
        bit::bits_to_binary_string(lowmc.encrypt(plaintext, key))
    );

        let lowmc = LowMC::new(164, 64, 80, 1);
        let plaintext = bit::bits_from_binary_string(
            "0000000000000000000000000000000000000000000000001111111111010101",
        );
        let key = bit::bits_from_binary_string(
            "00000000000000000000000000000000000000000000000000000000000000000000000000000001",
        );
        assert_eq!(
            "1111111011110001110100111110000000101011000000001011011110100000",
            bit::bits_to_binary_string(lowmc.encrypt(plaintext, key))
        );
    }
}
