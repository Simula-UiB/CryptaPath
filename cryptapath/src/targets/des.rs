use crate::bit::{Bit, *};
use crate::sbox::Sbox;
use crate::targets::Cipher;
use std::cell::RefCell;

pub struct DES {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox_tables: Vec<Vec<u8>>,
    expansion_table: [usize; 48],
    permutation_table: [usize; 32],
    sbox: RefCell<Sbox>,
}

impl DES {
    pub fn new(n_rounds: usize) -> Self {
        let sbox_tables = vec![
            vec![
                0xe, 0x0, 0x4, 0xf, 0xd, 0x7, 0x1, 0x4, 0x2, 0xe, 0xf, 0x2, 0xb, 0xd, 0x8, 0x1,
                0x3, 0xa, 0xa, 0x6, 0x6, 0xc, 0xc, 0xb, 0x5, 0x9, 0x9, 0x5, 0x0, 0x3, 0x7, 0x8,
                0x4, 0xf, 0x1, 0xc, 0xe, 0x8, 0x8, 0x2, 0xd, 0x4, 0x6, 0x9, 0x2, 0x1, 0xb, 0x7,
                0xf, 0x5, 0xc, 0xb, 0x9, 0x3, 0x7, 0xe, 0x3, 0xa, 0xa, 0x0, 0x5, 0x6, 0x0, 0xd,
            ],
            vec![
                0xf, 0x3, 0x1, 0xd, 0x8, 0x4, 0xe, 0x7, 0x6, 0xf, 0xb, 0x2, 0x3, 0x8, 0x4, 0xe,
                0x9, 0xc, 0x7, 0x0, 0x2, 0x1, 0xd, 0xa, 0xc, 0x6, 0x0, 0x9, 0x5, 0xb, 0xa, 0x5,
                0x0, 0xd, 0xd, 0x8, 0x7, 0xa, 0xb, 0x1, 0xa, 0x3, 0x4, 0xf, 0xd, 0x4, 0x1, 0x2,
                0x5, 0xb, 0x8, 0x6, 0xc, 0x7, 0x6, 0xc, 0x9, 0x0, 0x3, 0x5, 0x2, 0xe, 0xf, 0x9,
            ],
            vec![
                0xa, 0x1, 0x0, 0x7, 0x9, 0x0, 0xe, 0x9, 0x6, 0x3, 0x3, 0x4, 0xf, 0x6, 0x5, 0xa,
                0x1, 0x2, 0xd, 0x8, 0xc, 0x5, 0x7, 0xe, 0xb, 0xc, 0x4, 0xb, 0x2, 0xf, 0x8, 0x1,
                0xd, 0x1, 0x6, 0xa, 0x4, 0xd, 0x9, 0x0, 0x8, 0x6, 0xf, 0x9, 0x3, 0x8, 0x0, 0x7,
                0xb, 0x4, 0x1, 0xf, 0x2, 0xe, 0xc, 0x3, 0x5, 0xb, 0xa, 0x5, 0xe, 0x2, 0x7, 0xc,
            ],
            vec![
                0x7, 0xd, 0xd, 0x8, 0xe, 0xb, 0x3, 0x5, 0x0, 0x6, 0x6, 0xf, 0x9, 0x0, 0xa, 0x3,
                0x1, 0x4, 0x2, 0x7, 0x8, 0x2, 0x5, 0xc, 0xb, 0x1, 0xc, 0xa, 0x4, 0xe, 0xf, 0x9,
                0xa, 0x3, 0x6, 0xf, 0x9, 0x0, 0x0, 0x6, 0xc, 0xa, 0xb, 0x1, 0x7, 0xd, 0xd, 0x8,
                0xf, 0x9, 0x1, 0x4, 0x3, 0x5, 0xe, 0xb, 0x5, 0xc, 0x2, 0x7, 0x8, 0x2, 0x4, 0xe,
            ],
            vec![
                0x2, 0xe, 0xc, 0xb, 0x4, 0x2, 0x1, 0xc, 0x7, 0x4, 0xa, 0x7, 0xb, 0xd, 0x6, 0x1,
                0x8, 0x5, 0x5, 0x0, 0x3, 0xf, 0xf, 0xa, 0xd, 0x3, 0x0, 0x9, 0xe, 0x8, 0x9, 0x6,
                0x4, 0xb, 0x2, 0x8, 0x1, 0xc, 0xb, 0x7, 0xa, 0x1, 0xd, 0xe, 0x7, 0x2, 0x8, 0xd,
                0xf, 0x6, 0x9, 0xf, 0xc, 0x0, 0x5, 0x9, 0x6, 0xa, 0x3, 0x4, 0x0, 0x5, 0xe, 0x3,
            ],
            vec![
                0xc, 0xa, 0x1, 0xf, 0xa, 0x4, 0xf, 0x2, 0x9, 0x7, 0x2, 0xc, 0x6, 0x9, 0x8, 0x5,
                0x0, 0x6, 0xd, 0x1, 0x3, 0xd, 0x4, 0xe, 0xe, 0x0, 0x7, 0xb, 0x5, 0x3, 0xb, 0x8,
                0x9, 0x4, 0xe, 0x3, 0xf, 0x2, 0x5, 0xc, 0x2, 0x9, 0x8, 0x5, 0xc, 0xf, 0x3, 0xa,
                0x7, 0xb, 0x0, 0xe, 0x4, 0x1, 0xa, 0x7, 0x1, 0x6, 0xd, 0x0, 0xb, 0x8, 0x6, 0xd,
            ],
            vec![
                0x4, 0xd, 0xb, 0x0, 0x2, 0xb, 0xe, 0x7, 0xf, 0x4, 0x0, 0x9, 0x8, 0x1, 0xd, 0xa,
                0x3, 0xe, 0xc, 0x3, 0x9, 0x5, 0x7, 0xc, 0x5, 0x2, 0xa, 0xf, 0x6, 0x8, 0x1, 0x6,
                0x1, 0x6, 0x4, 0xb, 0xb, 0xd, 0xd, 0x8, 0xc, 0x1, 0x3, 0x4, 0x7, 0xa, 0xe, 0x7,
                0xa, 0x9, 0xf, 0x5, 0x6, 0x0, 0x8, 0xf, 0x0, 0xe, 0x5, 0x2, 0x9, 0x3, 0x2, 0xc,
            ],
            vec![
                0xd, 0x1, 0x2, 0xf, 0x8, 0xd, 0x4, 0x8, 0x6, 0xa, 0xf, 0x3, 0xb, 0x7, 0x1, 0x4,
                0xa, 0xc, 0x9, 0x5, 0x3, 0x6, 0xe, 0xb, 0x5, 0x0, 0x0, 0xe, 0xc, 0x9, 0x7, 0x2,
                0x7, 0x2, 0xb, 0x1, 0x4, 0xe, 0x1, 0x7, 0x9, 0x4, 0xc, 0xa, 0xe, 0x8, 0x2, 0xd,
                0x0, 0xf, 0x6, 0xc, 0xa, 0x9, 0xd, 0x0, 0xf, 0x3, 0x3, 0x5, 0x5, 0x6, 0x8, 0xb,
            ],
        ];
        let expansion_table = [
            31, 0, 1, 2, 3, 4, 3, 4, 5, 6, 7, 8, 7, 8, 9, 10, 11, 12, 11, 12, 13, 14, 15, 16, 15,
            16, 17, 18, 19, 20, 19, 20, 21, 22, 23, 24, 23, 24, 25, 26, 27, 28, 27, 28, 29, 30, 31,
            0,
        ];
        let permutation_table = [
            16, 7, 20, 21, 29, 12, 28, 17, 1, 15, 23, 26, 5, 18, 31, 10, 2, 8, 24, 14, 32, 27, 3,
            9, 19, 13, 30, 6, 22, 11, 4, 25,
        ];
        let message_length = 64;
        let key_length = 64;
        let init = sbox_tables[0].clone();
        DES {
            n_rounds,
            message_length,
            key_length,
            sbox_tables,
            expansion_table,
            permutation_table,
            sbox: RefCell::new(Sbox::new(6, 4, init, message_length + key_length)),
        }
    }

    fn f_function(&self, in_bits: Vec<Bit>, round_key: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), 32);
        assert_eq!(round_key.len(), 48);
        let mut expanded_bits = Vec::with_capacity(48);
        for bit in self.expansion_table.iter() {
            expanded_bits.push(in_bits[*bit].clone());
        }
        expanded_bits = bit_vector_xoring(expanded_bits, round_key);
        let mut post_sbox_bits = Vec::with_capacity(48);
        for sbox_index in 0..8 {
            post_sbox_bits.append(
                &mut self
                    .sbox
                    .borrow()
                    .apply(expanded_bits[sbox_index * 6..(sbox_index + 1) * 6].to_vec()),
            );
            self.sbox.replace(Sbox::replace_existing_sbox(
                6,
                4,
                self.sbox_tables[(sbox_index + 1) % 8].clone(),
                self.sbox.clone().into_inner(),
            ));
        }
        let mut out_bits = Vec::with_capacity(32);
        for bit in self.permutation_table.iter() {
            out_bits.push(post_sbox_bits[*bit - 1].clone());
        }
        out_bits
    }
    fn initial_permutation(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let initial_table: [usize; 64] = [
            58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4, 62, 54, 46, 38, 30, 22,
            14, 6, 64, 56, 48, 40, 32, 24, 16, 8, 57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27,
            19, 11, 3, 61, 53, 45, 37, 29, 21, 13, 5, 63, 55, 47, 39, 31, 23, 15, 7,
        ];
        let mut out_bits = Vec::with_capacity(self.message_length);
        for bit in initial_table.iter() {
            out_bits.push(in_bits[*bit - 1].clone())
        }
        out_bits
    }
    fn final_permutation(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let final_table: [usize; 64] = [
            40, 8, 48, 16, 56, 24, 64, 32, 39, 7, 47, 15, 55, 23, 63, 31, 38, 6, 46, 14, 54, 22,
            62, 30, 37, 5, 45, 13, 53, 21, 61, 29, 36, 4, 44, 12, 52, 20, 60, 28, 35, 3, 43, 11,
            51, 19, 59, 27, 34, 2, 42, 10, 50, 18, 58, 26, 33, 1, 41, 9, 49, 17, 57, 25,
        ];
        let mut out_bits = Vec::with_capacity(self.message_length);
        for bit in final_table.iter() {
            out_bits.push(in_bits[*bit - 1].clone())
        }
        out_bits
    }

    fn make_round_keys(&self, key: Vec<Bit>) -> Vec<Vec<Bit>> {
        let mut round_keys = Vec::with_capacity(16);
        let pc1_c_table: [usize; 28] = [
            57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2, 59, 51, 43, 35, 27, 19,
            11, 3, 60, 52, 44, 36,
        ];
        let pc1_d_table: [usize; 28] = [
            63, 55, 47, 39, 31, 23, 15, 7, 62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21,
            13, 5, 28, 20, 12, 4,
        ];
        let pc2_table: [usize; 48] = [
            14, 17, 11, 24, 1, 5, 3, 28, 15, 6, 21, 10, 23, 19, 12, 4, 26, 8, 16, 7, 27, 20, 13, 2,
            41, 52, 31, 37, 47, 55, 30, 40, 51, 45, 33, 48, 44, 49, 39, 56, 34, 53, 46, 42, 50, 36,
            29, 32,
        ];
        let mut c = Vec::with_capacity(28);
        let mut d = Vec::with_capacity(28);
        for bit in pc1_c_table.iter() {
            c.push(key[*bit - 1].clone());
        }
        for bit in pc1_d_table.iter() {
            d.push(key[*bit - 1].clone());
        }
        for round in 1..=self.n_rounds {
            let shift_number = match round {
                1 | 2 | 9 | 16 => 1,
                3 | 4 | 5 | 6 | 7 | 8 | 10 | 11 | 12 | 13 | 14 | 15 => 2,
                _ => panic!("number of round should be between 1 and 16"),
            };
            c = left_shift(c, shift_number);
            d = left_shift(d, shift_number);
            let mut tmp = c.clone();
            tmp.append(&mut d.clone());
            let mut round_key = Vec::with_capacity(48);
            for bit in pc2_table.iter() {
                round_key.push(tmp[*bit - 1].clone());
            }
            round_keys.push(round_key);
        }
        round_keys
    }
}

fn xor_l_r(l: Vec<Bit>, r: Vec<Bit>) -> Vec<Bit> {
    assert_eq!(l.len(), 32);
    assert_eq!(r.len(), 32);
    bit_vector_xoring(l, r)
}

fn left_shift(in_bit: Vec<Bit>, n_shift: usize) -> Vec<Bit> {
    let mut out_bits = in_bit.iter().cloned().skip(n_shift).collect::<Vec<Bit>>();
    out_bits.append(&mut in_bit.iter().cloned().take(n_shift).collect::<Vec<Bit>>());
    out_bits
}

impl Cipher for DES {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit> {
        let round_keys = self.make_round_keys(key_bits);
        let mut out_bits = in_bits.clone();
        out_bits = self.initial_permutation(out_bits);
        let mut l = out_bits.iter().cloned().take(32).collect::<Vec<Bit>>();
        let mut r = out_bits
            .iter()
            .cloned()
            .skip(32)
            .take(32)
            .collect::<Vec<Bit>>();
        for round in 0..self.n_rounds {
            let tmp = r.clone();
            r = xor_l_r(l, self.f_function(r, round_keys[round].clone()));
            l = tmp;
        }
        out_bits.clear();
        for bit in r.drain(..) {
            out_bits.push(bit);
        }
        for bit in l.drain(..) {
            out_bits.push(bit);
        }
        self.final_permutation(out_bits)
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
        self.sbox.borrow().clone()
    }
}

// from https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nbsspecialpublication500-20e1980.pdf
#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{des::DES, Cipher};
    #[test]
    fn validate_encrypt() {
        let des = DES::new(16);
        let message = bit::bits_from_hex_string("95f8a5e5dd31d900");
        let key = bit::bits_from_hex_string("0101010101010101");
        let ciphertext = des.encrypt(message, key);
        assert_eq!("8000000000000000", bit::bits_to_hex_string(ciphertext));

        let message = bit::bits_from_hex_string("0000000000000000");
        let key = bit::bits_from_hex_string("1007103489988020");
        let ciphertext = des.encrypt(message, key);
        assert_eq!("0c0cc00c83ea48fd", bit::bits_to_hex_string(ciphertext));

        let message = bit::bits_from_hex_string("42fd443059577fa2");
        let key = bit::bits_from_hex_string("04b915ba43feb5b6");
        let ciphertext = des.encrypt(message, key);
        assert_eq!("af37fb421f8c4095", bit::bits_to_hex_string(ciphertext));
    }
}
