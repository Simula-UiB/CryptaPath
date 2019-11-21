use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::{bit, bit::Bit, bit::*};

pub struct MiniAES2x2 {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox: Sbox,
}

impl MiniAES2x2 {
    pub fn new(n_rounds: usize) -> Self {
        let table = vec![
            0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7,
            0xab, 0x76, 0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf,
            0x9c, 0xa4, 0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5,
            0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a,
            0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e,
            0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed,
            0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf, 0xd0, 0xef,
            0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
            0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff,
            0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d,
            0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee,
            0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c,
            0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5,
            0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08, 0xba, 0x78, 0x25, 0x2e,
            0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70, 0x3e,
            0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
            0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55,
            0x28, 0xdf, 0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f,
            0xb0, 0x54, 0xbb, 0x16,
        ];
        MiniAES2x2 {
            n_rounds,
            message_length: 32,
            key_length: 32,
            sbox: Sbox::new(8, 8, table, 64),
        }
    }

    fn sub_bytes(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..4 {
            out_bits.append(&mut self.sbox.apply(in_bits[i * 8..(i + 1) * 8].to_vec()));
        }
        out_bits
    }

    fn shift_rows(&self, mut in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = in_bits[0..16].to_vec();
        out_bits.append(&mut in_bits[24..32].to_vec());
        out_bits.append(&mut in_bits[16..24].to_vec());
        out_bits
    }

    fn mix_columns(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let mut out_bits = vec![bit!(false); in_bits.len()];
        for column in 0..2 {
            let a = in_bits
                .iter()
                .cloned()
                .skip(column * 8)
                .take(8)
                .collect::<Vec<Bit>>();
            let b = in_bits
                .iter()
                .cloned()
                .skip(16 + column * 8)
                .take(8)
                .collect::<Vec<Bit>>();
            let a_x = Self::time_x(a.clone());
            let b_x = Self::time_x(b.clone());
            let out_up = {
                let a_x_1 = bit_vector_xoring(a_x.clone(), a);
                bit_vector_xoring(a_x_1, b_x.clone())
            };
            let out_down = {
                let b_x_1 = bit_vector_xoring(b_x, b);
                bit_vector_xoring(b_x_1, a_x)
            };
            for bit in 0..8 {
                out_bits[bit + column * 8] = out_up[bit].clone();
                out_bits[bit + column * 8 + 16] = out_down[bit].clone();
            }
        }
        out_bits
    }

    fn time_x(in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), 8);
        let mut time_x = in_bits[1..8].to_vec();
        time_x.push(in_bits[0].clone());
        time_x[3] ^= in_bits[0].clone();
        time_x[4] ^= in_bits[0].clone();
        time_x[6] ^= in_bits[0].clone();
        time_x
    }

    fn add_round_key(&self, in_bits: Vec<Bit>, round_key: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        assert!(round_key.len() == self.message_length);
        bit_vector_xoring(in_bits, round_key)
    }

    fn make_round_keys(&self, mut key: Vec<Bit>) -> Vec<Vec<Bit>> {
        assert_eq!(key.len(), self.key_length);
        let mut round_keys = Vec::with_capacity(self.n_rounds);
        let round_constants = vec![
            bit::bits_from_hex_string("0100"),
            bit::bits_from_hex_string("0200"),
            bit::bits_from_hex_string("0400"),
            bit::bits_from_hex_string("0800"),
            bit::bits_from_hex_string("1000"),
            bit::bits_from_hex_string("2000"),
            bit::bits_from_hex_string("4000"),
            bit::bits_from_hex_string("8000"),
            bit::bits_from_hex_string("1B00"),
            bit::bits_from_hex_string("3600"),
        ];
        let mut k0 = key[0..8].to_vec();
        k0.append(&mut key[16..24].to_vec());
        let mut k1 = key[8..16].to_vec();
        k1.append(&mut key[24..32].to_vec());
        round_keys.push(key);
        for round in 0..self.n_rounds {
            let k1_save = k1.clone();
            let mut rot = k1[8..16].to_vec();
            rot.append(&mut k1[0..8].to_vec());
            k1.clear();
            for i in 0..2 {
                k1.append(&mut self.sbox.apply(rot[i * 8..(i + 1) * 8].to_vec()));
            }
            k1 = bit_vector_xoring(k1, round_constants[round].clone());
            k0 = bit_vector_xoring(k0, k1);
            k1 = bit_vector_xoring(k0.clone(), k1_save);
            let mut round_key = k0[0..8].to_vec();
            round_key.append(&mut k1[0..8].to_vec());
            round_key.append(&mut k0[8..16].to_vec());
            round_key.append(&mut k1[8..16].to_vec());
            round_keys.push(round_key);
        }
        round_keys
    }
}

impl Cipher for MiniAES2x2 {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit> {
        let round_keys = self.make_round_keys(key_bits);
        let mut out_bits = in_bits.clone();
        out_bits = self.add_round_key(out_bits, round_keys[0].clone());
        for round_index in 0..self.n_rounds - 1 {
            out_bits = self.add_round_key(
                self.mix_columns(self.shift_rows(self.sub_bytes(out_bits))),
                round_keys[round_index + 1].clone(),
            );
        }
        self.add_round_key(
            self.shift_rows(self.sub_bytes(out_bits)),
            round_keys[self.n_rounds].clone(),
        )
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

// from http://doc.sagemath.org/html/en/reference/cryptography/sage/crypto/mq/sr.html
#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{miniaes2x2::MiniAES2x2, Cipher};

    
    #[test]
    fn validate_key_schedule() {
        let key = bit::bits_from_hex_string("c9bd6550");
        let cipher = MiniAES2x2::new(10);
        let round_keys = cipher.make_round_keys(key);
        let expected_keys = vec![
            "c9bd6550", "9b261f4f", "1d3be8a7", "457e0aad", "d8a6f954", "e84edd89", "6f21f27b",
            "0e2f0f74", "1c331a6e", "98abd9b7", "07acbb0c",
        ];
        for (key, expected_key) in round_keys
            .iter()
            .map(|key| bit::bits_to_hex_string(key.clone()))
            .zip(expected_keys.iter())
        {
            assert_eq!(key, *expected_key);
        }
    }

    #[test]
    fn validate_mix_column() {
        let state = bit::bits_from_hex_string("9c6904e1");
        let cipher = MiniAES2x2::new(10);
        let expected_state = "b7622fea";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.mix_columns(state))
        );

        let state = bit::bits_from_hex_string("b9bAc044");
        let cipher = MiniAES2x2::new(10);
        let expected_state = "4b5d32a3";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.mix_columns(state))
        );
    }

    #[test]
    fn validate_shift_rows() {
        let state = bit::bits_from_hex_string("a6767ea0");
        let cipher = MiniAES2x2::new(10);
        let expected_state = "a676a07e";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.shift_rows(state))
        );

        let state = bit::bits_from_hex_string("8fbcc647");
        let cipher = MiniAES2x2::new(10);
        let expected_state = "8fbc47c6";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.shift_rows(state))
        );
    }

    #[test]
    fn validate_sub_bytes() {
        let state = bit::bits_from_hex_string("16c59e64");
        let cipher = MiniAES2x2::new(10);
        let expected_state = "47a60b43";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.sub_bytes(state))
        );

        let state = bit::bits_from_hex_string("06abb7f1");
        let cipher = MiniAES2x2::new(10);
        let expected_state = "6f62a9a1";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.sub_bytes(state))
        );
    }

    #[test]
    fn validate_encrypt() {
        let cipher = MiniAES2x2::new(10);
        let key = bit::bits_from_hex_string("dc16b351");
        let plaintext = bit::bits_from_hex_string("0d2729ac");
        let expected_ciphertext = "56737333";
        assert_eq!(
            expected_ciphertext,
            bit::bits_to_hex_string(cipher.encrypt(plaintext, key))
        );
    }

}
