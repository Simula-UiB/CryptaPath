use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::{bit, bit::Bit, bit::*};

pub struct Skinny128 {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox: Sbox,
}

impl Skinny128 {
    pub fn new(key_length: usize, n_rounds: usize) -> Self {
        let message_length = 128;
        assert!(
            key_length == message_length
                || key_length == message_length * 2
                || key_length == message_length * 3
        );
        let table = vec![
            0x65, 0x4c, 0x6a, 0x42, 0x4b, 0x63, 0x43, 0x6b, 0x55, 0x75, 0x5a, 0x7a, 0x53, 0x73,
            0x5b, 0x7b, 0x35, 0x8c, 0x3a, 0x81, 0x89, 0x33, 0x80, 0x3b, 0x95, 0x25, 0x98, 0x2a,
            0x90, 0x23, 0x99, 0x2b, 0xe5, 0xcc, 0xe8, 0xc1, 0xc9, 0xe0, 0xc0, 0xe9, 0xd5, 0xf5,
            0xd8, 0xf8, 0xd0, 0xf0, 0xd9, 0xf9, 0xa5, 0x1c, 0xa8, 0x12, 0x1b, 0xa0, 0x13, 0xa9,
            0x05, 0xb5, 0x0a, 0xb8, 0x03, 0xb0, 0x0b, 0xb9, 0x32, 0x88, 0x3c, 0x85, 0x8d, 0x34,
            0x84, 0x3d, 0x91, 0x22, 0x9c, 0x2c, 0x94, 0x24, 0x9d, 0x2d, 0x62, 0x4a, 0x6c, 0x45,
            0x4d, 0x64, 0x44, 0x6d, 0x52, 0x72, 0x5c, 0x7c, 0x54, 0x74, 0x5d, 0x7d, 0xa1, 0x1a,
            0xac, 0x15, 0x1d, 0xa4, 0x14, 0xad, 0x02, 0xb1, 0x0c, 0xbc, 0x04, 0xb4, 0x0d, 0xbd,
            0xe1, 0xc8, 0xec, 0xc5, 0xcd, 0xe4, 0xc4, 0xed, 0xd1, 0xf1, 0xdc, 0xfc, 0xd4, 0xf4,
            0xdd, 0xfd, 0x36, 0x8e, 0x38, 0x82, 0x8b, 0x30, 0x83, 0x39, 0x96, 0x26, 0x9a, 0x28,
            0x93, 0x20, 0x9b, 0x29, 0x66, 0x4e, 0x68, 0x41, 0x49, 0x60, 0x40, 0x69, 0x56, 0x76,
            0x58, 0x78, 0x50, 0x70, 0x59, 0x79, 0xa6, 0x1e, 0xaa, 0x11, 0x19, 0xa3, 0x10, 0xab,
            0x06, 0xb6, 0x08, 0xba, 0x00, 0xb3, 0x09, 0xbb, 0xe6, 0xce, 0xea, 0xc2, 0xcb, 0xe3,
            0xc3, 0xeb, 0xd6, 0xf6, 0xda, 0xfa, 0xd3, 0xf3, 0xdb, 0xfb, 0x31, 0x8a, 0x3e, 0x86,
            0x8f, 0x37, 0x87, 0x3f, 0x92, 0x21, 0x9e, 0x2e, 0x97, 0x27, 0x9f, 0x2f, 0x61, 0x48,
            0x6e, 0x46, 0x4f, 0x67, 0x47, 0x6f, 0x51, 0x71, 0x5e, 0x7e, 0x57, 0x77, 0x5f, 0x7f,
            0xa2, 0x18, 0xae, 0x16, 0x1f, 0xa7, 0x17, 0xaf, 0x01, 0xb2, 0x0e, 0xbe, 0x07, 0xb7,
            0x0f, 0xbf, 0xe2, 0xca, 0xee, 0xc6, 0xcf, 0xe7, 0xc7, 0xef, 0xd2, 0xf2, 0xde, 0xfe,
            0xd7, 0xf7, 0xdf, 0xff,
        ];

        Skinny128 {
            n_rounds,
            message_length,
            key_length,
            sbox: Sbox::new(8, 8, table, message_length + key_length),
        }
    }

    fn sub_cells(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..16 {
            out_bits.append(&mut self.sbox.apply(in_bits[i * 8..(i + 1) * 8].to_vec()));
        }
        out_bits
    }

    fn add_constants(&self, in_bits: Vec<Bit>, round_index: usize) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = in_bits.clone();
        let (c0, c1) = add_constants_lfsr(round_index);
        let c2 = [
            bit!(false),
            bit!(false),
            bit!(false),
            bit!(false),
            bit!(false),
            bit!(false),
            bit!(true),
            bit!(false),
        ];
        let constants = [c0, c1, c2];
        for row in 0..3 {
            for bit in 0..8 {
                out_bits[bit + row * 32] ^= constants[row][bit].clone();
            }
        }
        out_bits
    }

    fn shift_rows(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for row in 0..4 {
            for column in 0..4 {
                for bit in 0..8 {
                    out_bits.push(in_bits[bit + ((column + 4 - row) % 4) * 8 + row * 4 * 8].clone())
                }
            }
        }
        out_bits
    }

    fn mix_columns(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = vec![bit!(false); self.message_length];
        for row_bit in 0..4 * 8 {
            out_bits[row_bit] = in_bits[row_bit].clone()
                ^ in_bits[8 * 8 + row_bit].clone()
                ^ in_bits[12 * 8 + row_bit].clone();
            out_bits[4 * 8 + row_bit] = in_bits[row_bit].clone();
            out_bits[8 * 8 + row_bit] =
                in_bits[4 * 8 + row_bit].clone() ^ in_bits[8 * 8 + row_bit].clone();
            out_bits[12 * 8 + row_bit] =
                in_bits[row_bit].clone() ^ in_bits[8 * 8 + row_bit].clone();
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
                for i in 0..self.message_length / 8 {
                    for bit in 0..8 {
                        tmp[i * 8 + bit] = tweakey_key[bit + permute_table[i] * 8].clone();
                    }
                }
                tweakey_key = tmp;
                // a 0 tweakey = TK1 in the spec
                match tweakey {
                    0 => (),
                    1 => {
                        let mut tmp = tweakey_key.clone();
                        for cell in 0..8 {
                            tmp[cell * 8] = tweakey_key[cell * 8 + 1].clone();
                            tmp[cell * 8 + 1] = tweakey_key[cell * 8 + 2].clone();
                            tmp[cell * 8 + 2] = tweakey_key[cell * 8 + 3].clone();
                            tmp[cell * 8 + 3] = tweakey_key[cell * 8 + 4].clone();
                            tmp[cell * 8 + 4] = tweakey_key[cell * 8 + 5].clone();
                            tmp[cell * 8 + 5] = tweakey_key[cell * 8 + 6].clone();
                            tmp[cell * 8 + 6] = tweakey_key[cell * 8 + 7].clone();
                            tmp[cell * 8 + 7] =
                                tweakey_key[cell * 8].clone() ^ tweakey_key[cell * 8 + 2].clone();
                        }
                        tweakey_key = tmp;
                    }
                    2 => {
                        let mut tmp = tweakey_key.clone();
                        for cell in 0..8 {
                            tmp[cell * 8] = tweakey_key[cell * 8 + 1].clone()
                                ^ tweakey_key[cell * 8 + 7].clone();
                            tmp[cell * 8 + 1] = tweakey_key[cell * 8].clone();
                            tmp[cell * 8 + 2] = tweakey_key[cell * 8 + 1].clone();
                            tmp[cell * 8 + 3] = tweakey_key[cell * 8 + 2].clone();
                            tmp[cell * 8 + 4] = tweakey_key[cell * 8 + 3].clone();
                            tmp[cell * 8 + 5] = tweakey_key[cell * 8 + 4].clone();
                            tmp[cell * 8 + 6] = tweakey_key[cell * 8 + 5].clone();
                            tmp[cell * 8 + 7] = tweakey_key[cell * 8 + 6].clone();
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

fn add_constants_lfsr(t: usize) -> ([Bit; 8], [Bit; 8]) {
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
    let c0 = [
        bit!(false),
        bit!(false),
        bit!(false),
        bit!(false),
        rc[3].clone(),
        rc[2].clone(),
        rc[1].clone(),
        rc[0].clone(),
    ];
    let c1 = [
        bit!(false),
        bit!(false),
        bit!(false),
        bit!(false),
        bit!(false),
        bit!(false),
        rc[5].clone(),
        rc[4].clone(),
    ];
    (c0, c1)
}

impl Cipher for Skinny128 {
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
    use crate::targets::{skinny128::Skinny128, Cipher};

    #[test]
    fn validate_encrypt() {
        //128-128
        let key = bit::bits_from_hex_string("4f55cfb0520cac52fd92c15f37073e93");
        let plaintext = bit::bits_from_hex_string("f20adb0eb08b648a3b2eeed1f0adda14");
        let expected_ciphertext = bit::bits_from_hex_string("22ff30d498ea62d7e45b476e33675b74");
        let skinny = Skinny128::new(128, 40);
        assert_eq!(expected_ciphertext, skinny.encrypt(plaintext, key));
        //128-256
        let key = bit::bits_from_hex_string(
            "009cec81605d4ac1d2ae9e3085d7a1f31ac123ebfc00fddcf01046ceeddfcab3",
        );
        let plaintext = bit::bits_from_hex_string("3a0c47767a26a68dd382a695e7022e25");
        let expected_ciphertext = bit::bits_from_hex_string("b731d98a4bde147a7ed4a6f16b9b587f");
        let skinny = Skinny128::new(256, 48);
        assert_eq!(expected_ciphertext, skinny.encrypt(plaintext, key));
        //128-384
        let key = bit::bits_from_hex_string("df889548cfc7ea52d296339301797449ab588a34a47f1ab2dfe9c8293fbea9a5ab1afac2611012cd8cef952618c3ebe8");
        let plaintext = bit::bits_from_hex_string("a3994b66ad85a3459f44e92b08f550cb");
        let expected_ciphertext = bit::bits_from_hex_string("94ecf589e2017c601b38c6346a10dcfa");
        let skinny = Skinny128::new(384, 56);
        assert_eq!(expected_ciphertext, skinny.encrypt(plaintext, key));
    }
}
