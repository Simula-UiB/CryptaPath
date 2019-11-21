use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::{bit, bit::Bit, bit::*};

pub struct Skinny64 {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox: Sbox,
}

impl Skinny64 {
    pub fn new(key_length: usize, n_rounds: usize) -> Self {
        let message_length = 64;
        assert!(
            key_length == message_length
                || key_length == message_length * 2
                || key_length == message_length * 3
        );
        let table = vec![
            0xc, 0x6, 0x9, 0x0, 0x1, 0xa, 0x2, 0xb, 0x3, 0x8, 0x5, 0xd, 0x4, 0xe, 0x7, 0xf,
        ];

        Skinny64 {
            n_rounds,
            message_length,
            key_length,
            sbox: Sbox::new(4, 4, table, message_length + key_length),
        }
    }

    fn sub_cells(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..16 {
            out_bits.append(&mut self.sbox.apply(in_bits[i * 4..(i + 1) * 4].to_vec()));
        }
        out_bits
    }

    fn add_constants(&self, in_bits: Vec<Bit>, round_index: usize) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = in_bits.clone();
        let (c0, c1) = add_constants_lfsr(round_index);
        let c2 = [bit!(false), bit!(false), bit!(true), bit!(false)];
        let constants = [c0, c1, c2];
        for row in 0..3 {
            for bit in 0..4 {
                out_bits[bit + row * 16] ^= constants[row][bit].clone();
            }
        }
        out_bits
    }

    fn shift_rows(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for row in 0..4 {
            for column in 0..4 {
                for bit in 0..4 {
                    out_bits.push(in_bits[bit + ((column + 4 - row) % 4) * 4 + row * 4 * 4].clone())
                }
            }
        }
        out_bits
    }

    fn mix_columns(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = vec![bit!(false); self.message_length];
        for row_bit in 0..4 * 4 {
            out_bits[row_bit] = in_bits[row_bit].clone()
                ^ in_bits[8 * 4 + row_bit].clone()
                ^ in_bits[12 * 4 + row_bit].clone();
            out_bits[4 * 4 + row_bit] = in_bits[row_bit].clone();
            out_bits[8 * 4 + row_bit] =
                in_bits[4 * 4 + row_bit].clone() ^ in_bits[8 * 4 + row_bit].clone();
            out_bits[12 * 4 + row_bit] =
                in_bits[row_bit].clone() ^ in_bits[8 * 4 + row_bit].clone();
        }
        out_bits
    }

    fn add_round_key(&self, in_bits: Vec<Bit>, round_key: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        assert!(round_key.len() == self.message_length);
        bit_vector_xoring(in_bits, round_key)
    }

    fn make_round_keys(&self, key: Vec<Bit>) -> Vec<Vec<Bit>> {
        assert!(key.len() == self.key_length);
        let permute_table = [9, 15, 8, 13, 10, 14, 12, 11, 0, 1, 2, 3, 4, 5, 6, 7];
        let mut round_keys = vec![vec![bit!(false); self.message_length]; self.n_rounds];
        for tweakey in 0..(self.key_length / self.message_length) {
            let mut tweakey_key =
                key[tweakey * self.message_length..(tweakey + 1) * self.message_length].to_vec();
            for round in 0..self.n_rounds {
                let mut r_key = tweakey_key.clone();
                r_key.truncate(self.message_length / 2);
                r_key.append(&mut vec![bit!(false); self.message_length / 2]);
                round_keys[round] = bit_vector_xoring(round_keys[round].clone(), r_key);
                let mut tmp = vec![bit!(false); self.message_length];
                for i in 0..self.message_length / 4 {
                    for bit in 0..4 {
                        tmp[i * 4 + bit] = tweakey_key[bit + permute_table[i] * 4].clone();
                    }
                }
                tweakey_key = tmp;
                // tweakey == 0 <=> TK1 in the spec
                match tweakey {
                    0 => (),
                    1 => {
                        let mut tmp = tweakey_key.clone();
                        for cell in 0..8 {
                            tmp[cell * 4] = tweakey_key[cell * 4 + 1].clone();
                            tmp[cell * 4 + 1] = tweakey_key[cell * 4 + 2].clone();
                            tmp[cell * 4 + 2] = tweakey_key[cell * 4 + 3].clone();
                            tmp[cell * 4 + 3] =
                                tweakey_key[cell * 4].clone() ^ tweakey_key[cell * 4 + 1].clone();
                        }
                        tweakey_key = tmp;
                    }
                    2 => {
                        let mut tmp = tweakey_key.clone();
                        for cell in 0..8 {
                            tmp[cell * 4] =
                                tweakey_key[cell * 4].clone() ^ tweakey_key[cell * 4 + 3].clone();
                            tmp[cell * 4 + 1] = tweakey_key[cell * 4].clone();
                            tmp[cell * 4 + 2] = tweakey_key[cell * 4 + 1].clone();
                            tmp[cell * 4 + 3] = tweakey_key[cell * 4 + 2].clone();
                        }
                        tweakey_key = tmp;
                    }
                    _ => panic!("more than 3 tweakey words is impossible"),
                };
            }
        }
        round_keys
    }
}

fn add_constants_lfsr(t: usize) -> ([Bit; 4], [Bit; 4]) {
    let mut rc = vec![bit!(false); 6];
    for _ in 0..=t {
        rc = vec![
            rc[5].clone() ^ rc[4].clone() ^ bit!(true),
            rc[0].clone(),
            rc[1].clone(),
            rc[2].clone(),
            rc[3].clone(),
            rc[4].clone(),
        ];
    }
    let c0 = [rc[3].clone(), rc[2].clone(), rc[1].clone(), rc[0].clone()];
    let c1 = [bit!(false), bit!(false), rc[5].clone(), rc[4].clone()];
    (c0, c1)
}

impl Cipher for Skinny64 {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit> {
        let round_keys = self.make_round_keys(key_bits);
        let mut out_bits = in_bits.clone();
        for round_index in 0..self.n_rounds {
            out_bits = self.mix_columns(self.shift_rows(self.add_round_key(
                self.add_constants(self.sub_cells(out_bits), round_index),
                round_keys[round_index].clone(),
            )));
        }
        out_bits
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

// from https://eprint.iacr.org/2016/660.pdf

#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{skinny64::Skinny64, Cipher};

    #[test]
    fn validate_encrypt() {
        //64-64
        let key = bit::bits_from_hex_string("f5269826fc681238");
        let plaintext = bit::bits_from_hex_string("06034f957724d19d");
        let expected_ciphertext = bit::bits_from_hex_string("bb39dfb2429b8ac7");
        let skinny = Skinny64::new(64, 32);
        assert_eq!(expected_ciphertext, skinny.encrypt(plaintext, key));
        //64-128
        let key = bit::bits_from_hex_string("9eb93640d088da6376a39d1c8bea71e1");
        let plaintext = bit::bits_from_hex_string("cf16cfe8fd0f98aa");
        let expected_ciphertext = bit::bits_from_hex_string("6ceda1f43de92b9e");
        let skinny = Skinny64::new(128, 36);
        assert_eq!(expected_ciphertext, skinny.encrypt(plaintext, key));
        //64-192
        let key = bit::bits_from_hex_string("ed00c85b120d68618753e24bfd908f60b2dbb41b422dfcd0");
        let plaintext = bit::bits_from_hex_string("530c61d35e8663c3");
        let expected_ciphertext = bit::bits_from_hex_string("dd2cf1a8f330303c");
        let skinny = Skinny64::new(192, 40);
        assert_eq!(expected_ciphertext, skinny.encrypt(plaintext, key));
    }
}
