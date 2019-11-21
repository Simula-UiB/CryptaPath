use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::{bit, bit::Bit, bit::*};
use std::cell::RefCell;

pub struct Prince {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    constants: Vec<Vec<Bit>>,
    inv_table: Vec<u8>,
    m_prime: Vec<String>,
    whitening: bool,
    sbox: RefCell<Sbox>,
}

macro_rules! binary_matrix {
        [$([$head:expr;$row:expr;$tail:expr]),*] => {
        {
            let mut mat = Vec::new();
        $(
            let mut tmp = "".to_string();
            tmp.push_str(&str::repeat("0",$head));
            tmp.push_str($row);
            tmp.push_str(&str::repeat("0",$tail));
            mat.push(tmp);
        )*
        mat
        }
    };
}

impl Prince {
    pub fn new(n_rounds: usize, whitening: bool) -> Self {
        assert!(
            n_rounds % 2 == 0,
            "to preserve the structure of prince, the number of round should be even"
        );
        assert!(n_rounds <= 12);
        let table = vec![
            0xb, 0xf, 0x3, 0x2, 0xa, 0xc, 0x9, 0x1, 0x6, 0x7, 0x8, 0x0, 0xe, 0x5, 0xd, 0x4,
        ];
        let inv_table = vec![
            0xb, 0x7, 0x3, 0x2, 0xf, 0xd, 0x8, 0x9, 0xa, 0x6, 0x4, 0x0, 0x5, 0xe, 0xc, 0x1,
        ];
        let message_length = 64;
        let key_length = if whitening { 128 } else { 64 };
        let constants = vec![
            bit::bits_from_hex_string("0000000000000000"),
            bit::bits_from_hex_string("13198a2e03707344"),
            bit::bits_from_hex_string("a4093822299f31d0"),
            bit::bits_from_hex_string("082efa98ec4e6c89"),
            bit::bits_from_hex_string("452821e638d01377"),
            bit::bits_from_hex_string("be5466cf34e90c6c"),
            bit::bits_from_hex_string("7ef84f78fd955cb1"),
            bit::bits_from_hex_string("85840851f1ac43aa"),
            bit::bits_from_hex_string("c882d32f25323c54"),
            bit::bits_from_hex_string("64a51195e0e3610d"),
            bit::bits_from_hex_string("d3b5a399ca0c2399"),
            bit::bits_from_hex_string("c0ac29b7c97c50dd"),
        ];
        let m_prime = binary_matrix![
        //M0
        [0;"0000100010001000";48],
        [0;"0100000001000100";48],
        [0;"0010001000000010";48],
        [0;"0001000100010000";48],
        [0;"1000100010000000";48],
        [0;"0000010001000100";48],
        [0;"0010000000100010";48],
        [0;"0001000100000001";48],
        [0;"1000100000001000";48],
        [0;"0100010001000000";48],
        [0;"0000001000100010";48],
        [0;"0001000000010001";48],
        [0;"1000000010001000";48],
        [0;"0100010000000100";48],
        [0;"0010001000100000";48],
        [0;"0000000100010001";48],
        //M1
        [16;"1000100010000000";32],
        [16;"0000010001000100";32],
        [16;"0010000000100010";32],
        [16;"0001000100000001";32],
        [16;"1000100000001000";32],
        [16;"0100010001000000";32],
        [16;"0000001000100010";32],
        [16;"0001000000010001";32],
        [16;"1000000010001000";32],
        [16;"0100010000000100";32],
        [16;"0010001000100000";32],
        [16;"0000000100010001";32],
        [16;"0000100010001000";32],
        [16;"0100000001000100";32],
        [16;"0010001000000010";32],
        [16;"0001000100010000";32],
        //M1
        [32;"1000100010000000";16],
        [32;"0000010001000100";16],
        [32;"0010000000100010";16],
        [32;"0001000100000001";16],
        [32;"1000100000001000";16],
        [32;"0100010001000000";16],
        [32;"0000001000100010";16],
        [32;"0001000000010001";16],
        [32;"1000000010001000";16],
        [32;"0100010000000100";16],
        [32;"0010001000100000";16],
        [32;"0000000100010001";16],
        [32;"0000100010001000";16],
        [32;"0100000001000100";16],
        [32;"0010001000000010";16],
        [32;"0001000100010000";16],
        //M0
        [48;"0000100010001000";0],
        [48;"0100000001000100";0],
        [48;"0010001000000010";0],
        [48;"0001000100010000";0],
        [48;"1000100010000000";0],
        [48;"0000010001000100";0],
        [48;"0010000000100010";0],
        [48;"0001000100000001";0],
        [48;"1000100000001000";0],
        [48;"0100010001000000";0],
        [48;"0000001000100010";0],
        [48;"0001000000010001";0],
        [48;"1000000010001000";0],
        [48;"0100010000000100";0],
        [48;"0010001000100000";0],
        [48;"0000000100010001";0]
        ];
        Prince {
            n_rounds,
            message_length,
            key_length,
            constants,
            inv_table,
            m_prime,
            whitening,
            sbox: RefCell::new(Sbox::new(4, 4, table, message_length + key_length)),
        }
    }

    fn add_round_key(&self, in_bits: Vec<Bit>, round_key: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        assert!(round_key.len() == self.message_length);
        bit_vector_xoring(in_bits, round_key)
    }

    fn add_constant(&self, in_bits: Vec<Bit>, round_index: usize) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        bit_vector_xoring(in_bits, self.constants[round_index].clone())
    }

    fn sbox_layer(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..16 {
            out_bits.append(
                &mut self
                    .sbox
                    .borrow()
                    .apply(in_bits[i * 4..(i + 1) * 4].to_vec()),
            );
        }
        out_bits
    }

    fn m_layer(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        let tmp = multiply_with_gf2_matrix(&self.m_prime, &in_bits);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for row in 0..4 {
            for column in 0..4 {
                for bit in 0..4 {
                    out_bits.push(tmp[bit + ((column * 5 + row * 4) % 16) * 4].clone())
                }
            }
        }
        out_bits
    }
    fn m_layer_inv(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        let mut out_bits = Vec::with_capacity(self.message_length);
        for row in 0..4 {
            for column in 0..4 {
                for bit in 0..4 {
                    out_bits.push(in_bits[bit + ((16 - column * 3 + row * 4) % 16) * 4].clone())
                }
            }
        }
        multiply_with_gf2_matrix(&self.m_prime, &out_bits)
    }

    fn m_prime_layer(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        multiply_with_gf2_matrix(&self.m_prime, &in_bits)
    }

    fn make_round_keys(&self, key: Vec<Bit>) -> Vec<Vec<Bit>> {
        assert!(key.len() == self.key_length);
        let (k0, k1, k0_prime) = match self.key_length {
            128 => {
                let k0 = key.iter().cloned().take(64).collect::<Vec<Bit>>();
                let k1 = key.iter().cloned().skip(64).take(64).collect::<Vec<Bit>>();
                let mut k0_prime = vec![k0[63].clone()];
                k0_prime.append(&mut k0.iter().cloned().take(63).collect());
                k0_prime[63] ^= k0[0].clone();
                (k0, k1, k0_prime)
            }
            64 => (key.clone(), key.clone(), key),
            _ => panic!("size of key should be 64 or 128"),
        };
        let mut round_keys = Vec::new();
        round_keys.push(k0);
        round_keys.push(k1);
        round_keys.push(k0_prime);
        round_keys
    }
}

fn multiply_with_gf2_matrix(matrix: &[String], in_bits: &[Bit]) -> Vec<Bit> {
    let mut out_bits = Vec::with_capacity(in_bits.len());
    for row in matrix {
        let r = row.chars().collect::<Vec<char>>();
        let mut tmp = bit!(false);
        for column in 0..64 {
            match r[column] {
                '1' => tmp ^= in_bits[column].clone(),
                '0' => (),
                _ => panic!("non binary character in binary string"),
            };
        }
        out_bits.push(tmp)
    }
    out_bits
}

impl Cipher for Prince {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit> {
        let round_keys = self.make_round_keys(key_bits);
        let mut out_bits = in_bits.clone();
        if self.whitening {
            out_bits = self.add_round_key(out_bits, round_keys[0].clone());
        }
        //Prince-core
        out_bits = self.add_constant(self.add_round_key(out_bits, round_keys[1].clone()), 0);
        for round in 1..self.n_rounds / 2 {
            out_bits = self.add_round_key(
                self.add_constant(self.m_layer(self.sbox_layer(out_bits)), round),
                round_keys[1].clone(),
            );
        }
        out_bits = self.m_prime_layer(self.sbox_layer(out_bits));
        self.sbox.replace(Sbox::replace_existing_sbox(
            4,
            4,
            self.inv_table.clone(),
            self.sbox.clone().into_inner(),
        ));
        out_bits = self.sbox_layer(out_bits);
        // Following the recommendation from the paper the reduced rounds are keeping the middle
        // symetry in an inside-out fashion
        // If I have 4 rounds, I will add the constants RC0, RC1, RC10 and RC11 for the encryption
        for (i, _) in (self.n_rounds / 2..self.n_rounds - 1).enumerate() {
            out_bits = self.sbox_layer(self.m_layer_inv(self.add_constant(
                self.add_round_key(out_bits, round_keys[1].clone()),
                12 - (self.n_rounds / 2) + i,
            )));
        }
        out_bits = self.add_round_key(self.add_constant(out_bits, 11), round_keys[1].clone());
        if self.whitening {
            out_bits = self.add_round_key(out_bits, round_keys[2].clone())
        }

        // We put back the original S-Box for future encryption using the same cipher
        self.sbox.replace(Sbox::replace_existing_sbox(
            4,
            4,
            vec![
                0xb, 0xf, 0x3, 0x2, 0xa, 0xc, 0x9, 0x1, 0x6, 0x7, 0x8, 0x0, 0xe, 0x5, 0xd, 0x4,
            ],
            self.sbox.clone().into_inner(),
        ));
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
        self.sbox.borrow().clone()
    }
}

// from https://eprint.iacr.org/2012/529.pdf

#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{prince::Prince, Cipher};

    #[test]
    fn validate_encrypt() {
        let prince = Prince::new(12, true);
        let message = bit::bits_from_hex_string("0000000000000000");
        let key = bit::bits_from_hex_string("00000000000000000000000000000000");
        let ciphertext = prince.encrypt(message, key);
        assert_eq!("818665aa0d02dfda", bit::bits_to_hex_string(ciphertext));

        let prince = Prince::new(12, true);
        let message = bit::bits_from_hex_string("ffffffffffffffff");
        let key = bit::bits_from_hex_string("00000000000000000000000000000000");
        let ciphertext = prince.encrypt(message, key);
        assert_eq!("604ae6ca03c20ada", bit::bits_to_hex_string(ciphertext));

        let prince = Prince::new(12, true);
        let message = bit::bits_from_hex_string("0000000000000000");
        let key = bit::bits_from_hex_string("ffffffffffffffff0000000000000000");
        let ciphertext = prince.encrypt(message, key);
        assert_eq!("9fb51935fc3df524", bit::bits_to_hex_string(ciphertext));

        let prince = Prince::new(12, true);
        let message = bit::bits_from_hex_string("0000000000000000");
        let key = bit::bits_from_hex_string("0000000000000000ffffffffffffffff");
        let ciphertext = prince.encrypt(message, key);
        assert_eq!("78a54cbe737bb7ef", bit::bits_to_hex_string(ciphertext));

        let prince = Prince::new(12, true);
        let message = bit::bits_from_hex_string("0123456789abcdef");
        let key = bit::bits_from_hex_string("0000000000000000fedcba9876543210");
        let ciphertext = prince.encrypt(message, key);
        assert_eq!("ae25ad3ca8fa9ccf", bit::bits_to_hex_string(ciphertext));
    }
}
