use crate::sbox::Sbox;
use crate::targets::Cipher;
use crate::{bit,bit::*, bit::Bit};

pub struct Present80 {
    n_rounds: usize,
    message_length: usize,
    key_length: usize,
    sbox: Sbox,
    p_layer: Vec<usize>,
}

impl Present80 {
    pub fn new(n_rounds: usize) -> Self {
        let table = vec![
            0xc, 0x5, 0x6, 0xb, 0x9, 0x0, 0xa, 0xd, 0x3, 0xe, 0xf, 0x8, 0x4, 0x7, 0x1, 0x2,
        ];
        let message_length = 64;
        let key_length = 80;
        let p_layer = vec![
            0, 16, 32, 48, 1, 17, 33, 49, 2, 18, 34, 50, 3, 19, 35, 51, 4, 20, 36, 52, 5, 21, 37,
            53, 6, 22, 38, 54, 7, 23, 39, 55, 8, 24, 40, 56, 9, 25, 41, 57, 10, 26, 42, 58, 11, 27,
            43, 59, 12, 28, 44, 60, 13, 29, 45, 61, 14, 30, 46, 62, 15, 31, 47, 63,
        ];
        Present80 {
            n_rounds,
            message_length,
            key_length,
            sbox: Sbox::new(4, 4, table, message_length + key_length),
            p_layer,
        }
    }

    fn p_layer(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let mut out_bits = vec![bit!(false);64];
        for (i,in_bit) in in_bits.iter().enumerate() {
            let perm = self.p_layer[i];
            out_bits[perm] = in_bit.clone();
        }
        out_bits
    }

    fn sbox_layer(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert_eq!(in_bits.len(), self.message_length);
        let mut out_bits = Vec::with_capacity(self.message_length);
        for i in 0..16 {
            out_bits.append(&mut self.sbox.apply(in_bits[i * 4..(i + 1) * 4].to_vec()));
        }
        out_bits
    }

    fn add_round_key(&self, in_bits: Vec<Bit>, round_key: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.message_length);
        assert!(round_key.len() == self.message_length);
        bit_vector_xoring(in_bits,round_key)
    }

    fn make_round_keys(&self, mut key: Vec<Bit>) -> Vec<Vec<Bit>> {
        assert!(key.len() == self.key_length);
        let mut round_keys = Vec::new();
        round_keys.push(key.iter().cloned().take(64).collect());
        for round in 1..=self.n_rounds {
            let mut left_part = key.iter().cloned().take(61).collect();
            key = key.iter().cloned().skip(61).collect();
            key.append(&mut left_part);
            let box_part = self.sbox.apply(key[0..4].to_vec());
            key[..4].clone_from_slice(&box_part[..4]);
            let round_counter = bit::bits_from_binary_string(&format!("{:05b}",round));
            for bit in 0..5 {
                key[60+bit] ^= round_counter[bit].clone();
            }
            round_keys.push(key.iter().cloned().take(64).collect());
        }
        round_keys
    }
}

impl Cipher for Present80 {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit> {
        let round_keys = self.make_round_keys(key_bits);
        let mut out_bits = in_bits.clone();
        for round_index in 0..self.n_rounds {
            out_bits = self.p_layer(
                self.sbox_layer(self.add_round_key(out_bits, round_keys[round_index].clone())),
            );
        }
        self.add_round_key(out_bits, round_keys[self.n_rounds].clone())
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

// from https://link.springer.com/content/pdf/10.1007%2F978-3-540-74735-2_31.pdf

#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{present80::Present80, Cipher};

#[test]
fn validate_encrypt() {
    let present = Present80::new(31);
    let message = bit::bits_from_hex_string("0000000000000000");
    let key = bit::bits_from_hex_string("00000000000000000000");
    let ciphertext = present.encrypt(message, key);
    assert_eq!("5579c1387b228445", bit::bits_to_hex_string(ciphertext));

    let message = bit::bits_from_hex_string("0000000000000000");
    let key = bit::bits_from_hex_string("FFFFFFFFFFFFFFFFFFFF");
    let ciphertext = present.encrypt(message, key);
    assert_eq!("e72c46c0f5945049", bit::bits_to_hex_string(ciphertext));

    let message = bit::bits_from_hex_string("FFFFFFFFFFFFFFFF");
    let key = bit::bits_from_hex_string("00000000000000000000");
    let ciphertext = present.encrypt(message, key);
    assert_eq!("a112ffc72f68417b", bit::bits_to_hex_string(ciphertext));

    let message = bit::bits_from_hex_string("FFFFFFFFFFFFFFFF");
    let key = bit::bits_from_hex_string("FFFFFFFFFFFFFFFFFFFF");
    let ciphertext = present.encrypt(message, key);
    assert_eq!("3333dcd3213210d2", bit::bits_to_hex_string(ciphertext));
}
}