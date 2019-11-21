use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::{bit, bit::Bit, bit::*};

pub struct MiniAES4x4 {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox: Sbox,
}

impl MiniAES4x4 {
    pub fn new(n_rounds: usize) -> Self {
        let table = vec![
            0x06, 0x0b, 0x05, 0x04, 0x02, 0x0e, 0x07, 0x0a, 0x09, 0x0d, 0x0f, 0x0c, 0x03, 0x01,
            0x00, 0x08,
        ];
        MiniAES4x4 {
            n_rounds,
            message_length: 64,
            key_length: 64,
            sbox: Sbox::new(4, 4, table, 128),
        }
    }

    fn sub_bytes(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..16 {
            out_bits.append(&mut self.sbox.apply(in_bits[i * 4..(i + 1) * 4].to_vec()));
        }
        out_bits
    }

    fn shift_rows(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for row in 0..4 {
            for column in 0..4 {
                for bit in 0..4 {
                    out_bits.push(in_bits[bit + ((column + row) % 4) * 4 + row * 4 * 4].clone())
                }
            }
        }
        out_bits
    }

    fn mix_columns(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let mut out_bits = vec![bit!(false); in_bits.len()];
        for column in 0..4 {
            let a = in_bits
                .iter()
                .cloned()
                .skip(column * 4)
                .take(4)
                .collect::<Vec<Bit>>();
            let b = in_bits
                .iter()
                .cloned()
                .skip(16 + column * 4)
                .take(4)
                .collect::<Vec<Bit>>();
            let c = in_bits
                .iter()
                .cloned()
                .skip(32 + column * 4)
                .take(4)
                .collect::<Vec<Bit>>();
            let d = in_bits
                .iter()
                .cloned()
                .skip(48 + column * 4)
                .take(4)
                .collect::<Vec<Bit>>();
            let a_x = Self::time_x(a.clone());
            let b_x = Self::time_x(b.clone());
            let c_x = Self::time_x(c.clone());
            let d_x = Self::time_x(d.clone());
            let out_a = {
                let b_x_1 = bit_vector_xoring(b_x.clone(), b.clone());
                let mut tmp = bit_vector_xoring(a_x.clone(), b_x_1);
                tmp = bit_vector_xoring(tmp, c.clone());
                bit_vector_xoring(tmp, d.clone())
            };
            let out_b = {
                let c_x_1 = bit_vector_xoring(c_x.clone(), c.clone());
                let mut tmp = bit_vector_xoring(a.clone(), b_x);
                tmp = bit_vector_xoring(tmp, c_x_1);
                bit_vector_xoring(tmp, d.clone())
            };
            let out_c = {
                let d_x_1 = bit_vector_xoring(d_x.clone(), d);
                let mut tmp = bit_vector_xoring(a.clone(), b.clone());
                tmp = bit_vector_xoring(tmp, c_x);
                bit_vector_xoring(tmp, d_x_1)
            };
            let out_d = {
                let a_x_1 = bit_vector_xoring(a_x, a);
                let mut tmp = bit_vector_xoring(a_x_1, b);
                tmp = bit_vector_xoring(tmp, c);
                bit_vector_xoring(tmp, d_x)
            };
            for bit in 0..4 {
                out_bits[bit + column * 4] = out_a[bit].clone();
                out_bits[bit + column * 4 + 16] = out_b[bit].clone();
                out_bits[bit + column * 4 + 32] = out_c[bit].clone();
                out_bits[bit + column * 4 + 48] = out_d[bit].clone();
            }
        }
        out_bits
    }

    fn time_x(in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), 4);
        let mut time_x = in_bits[1..4].to_vec();
        time_x.push(in_bits[0].clone());
        time_x[2] ^= in_bits[0].clone();
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
            bit::bits_from_hex_string("1000"),
            bit::bits_from_hex_string("2000"),
            bit::bits_from_hex_string("4000"),
            bit::bits_from_hex_string("8000"),
            bit::bits_from_hex_string("3000"),
            bit::bits_from_hex_string("6000"),
            bit::bits_from_hex_string("c000"),
            bit::bits_from_hex_string("b000"),
            bit::bits_from_hex_string("5000"),
            bit::bits_from_hex_string("a000"),
        ];
        let (mut k0, mut k1, mut k2, mut k3) = (
            key[0..4].to_vec(),
            key[4..8].to_vec(),
            key[8..12].to_vec(),
            key[12..16].to_vec(),
        );
        for column in 1..4 {
            let start_row = column * 16;
            k0.append(&mut key[start_row..start_row + 4].to_vec());
            k1.append(&mut key[start_row + 4..start_row + 8].to_vec());
            k2.append(&mut key[start_row + 8..start_row + 12].to_vec());
            k3.append(&mut key[start_row + 12..start_row + 16].to_vec());
        }
        round_keys.push(key);
        for round in 0..self.n_rounds {
            let k3_save = k3.clone();
            let mut rot = k3[4..16].to_vec();
            rot.append(&mut k3[0..4].to_vec());
            k3.clear();
            for i in 0..4 {
                k3.append(&mut self.sbox.apply(rot[i * 4..(i + 1) * 4].to_vec()));
            }
            k3 = bit_vector_xoring(k3, round_constants[round].clone());
            k0 = bit_vector_xoring(k0, k3);
            k1 = bit_vector_xoring(k0.clone(), k1);
            k2 = bit_vector_xoring(k1.clone(), k2);
            k3 = bit_vector_xoring(k2.clone(), k3_save);
            let mut round_key = k0[0..4].to_vec();
            round_key.append(&mut k1[0..4].to_vec());
            round_key.append(&mut k2[0..4].to_vec());
            round_key.append(&mut k3[0..4].to_vec());
            for column in 1..4 {
                let start = column * 4;
                round_key.append(&mut k0[start..start + 4].to_vec());
                round_key.append(&mut k1[start..start + 4].to_vec());
                round_key.append(&mut k2[start..start + 4].to_vec());
                round_key.append(&mut k3[start..start + 4].to_vec());
            }
            round_keys.push(round_key);
        }
        round_keys
    }
}

impl Cipher for MiniAES4x4 {
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

#[test]
fn validate_key_schedule() {
    let key = bit::bits_from_hex_string("c4de2cadef240c95");
    let cipher = MiniAES4x4::new(10);
    let round_keys = cipher.make_round_keys(key);
    let expected_keys = vec![
        "c4de2cadef240c95",
        "c85b0c6b0fd90c50",
        "2af4d17c694dc055",
        "5f04cda68158eebe",
        "a551582489c4c297",
        "beba7fd92b7375cb",
        "0e5f3c18e5218d1a",
        "5be1845d14670dc6",
        "f4ab263e6243b6ac",
        "ae4f603d573071b7",
        "1fb4003ef8bbfe52",
    ];
    for (key, expected_key) in round_keys
        .iter()
        .map(|key| bit::bits_to_hex_string(key.clone()))
        .zip(expected_keys.iter())
    {
        assert_eq!(key, *expected_key);
    }
}

//from http://doc.sagemath.org/html/en/reference/cryptography/sage/crypto/mq/sr.html

#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{miniaes4x4::MiniAES4x4, Cipher};

    #[test]
    fn validate_mix_column() {
        let state = bit::bits_from_hex_string("b316c65d45d0e76d");
        let cipher = MiniAES4x4::new(10);
        let expected_state = "8e652792e67f987e";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.mix_columns(state))
        );

        let state = bit::bits_from_hex_string("7865690c18f0f9ad");
        let expected_state = "aa907be31a6d3b2a";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.mix_columns(state))
        );
    }

    #[test]
    fn validate_shift_rows() {
        let state = bit::bits_from_hex_string("b5da6ba15149d885");
        let cipher = MiniAES4x4::new(10);
        let expected_state = "b5daba1649515d88";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.shift_rows(state))
        );

        let state = bit::bits_from_hex_string("8746923890145362");
        let expected_state = "8746238914902536";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.shift_rows(state))
        );
    }

    #[test]
    fn validate_sub_bytes() {
        let state = bit::bits_from_hex_string("fc7979b7f955da1d");
        let cipher = MiniAES4x4::new(10);
        let expected_state = "83adadca8dee1fb1";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.sub_bytes(state))
        );

        let state = bit::bits_from_hex_string("736e20605aa1064d");
        let expected_state = "a4705676effb6721";
        assert_eq!(
            expected_state,
            bit::bits_to_hex_string(cipher.sub_bytes(state))
        );
    }

    #[test]
    fn validate_encrypt() {
        let cipher = MiniAES4x4::new(10);
        let key = bit::bits_from_hex_string("07f5167304421207");
        let plaintext = bit::bits_from_hex_string("05f6a0b7035625dd");
        let expected_ciphertext = "336dc64ef859c8c4";
        assert_eq!(
            expected_ciphertext,
            bit::bits_to_hex_string(cipher.encrypt(plaintext, key))
        );
    }
}
