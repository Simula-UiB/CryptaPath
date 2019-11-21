use crate::sbox::Sbox;
use crate::targets::SpongeHash;
use crate::{bit, bit::Bit};

pub struct Keccak {
    n_rounds: usize,
    message_length: usize,
    output_length: usize,
    rate: usize,
    capacity: usize,
    chi_sbox: Sbox,
}

impl Keccak {
    pub fn new(
        n_rounds: usize,
        message_length: usize,
        output_length: usize,
        rate: usize,
        capacity: usize,
    ) -> Self {
        let table = vec![
            0x00, 0x05, 0x0a, 0x0b, 0x14, 0x11, 0x16, 0x17, 0x09, 0x0c, 0x03, 0x02, 0x0d, 0x08,
            0x0f, 0x0e, 0x12, 0x15, 0x18, 0x1b, 0x06, 0x01, 0x04, 0x07, 0x1a, 0x1d, 0x10, 0x13,
            0x1e, 0x19, 0x1c, 0x1f,
        ];
        assert!((rate + capacity) % 25 == 0);
        Keccak {
            n_rounds,
            message_length,
            output_length,
            rate,
            capacity,
            chi_sbox: Sbox::new(5, 5, table, message_length),
        }
    }

    fn minus_one_mod_z(input: usize, z: usize) -> usize {
        if input == 0 {
            z - 1
        } else {
            input - 1
        }
    }

    pub fn theta(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.state_length());
        let w = in_bits.len() / 25;
        let mut c = Vec::with_capacity(5 * w);
        for z in 0..w {
            for x in 0..5 {
                let mut tmp = bit!(false);
                for y in 0..5 {
                    tmp ^= in_bits[x + y * 5 + z * 25].clone();
                }
                c.push(tmp);
            }
        }
        let mut d = Vec::with_capacity(5 * w);
        for z in 0..w {
            for x in 0..5 {
                d.push(
                    c[Self::minus_one_mod_z(x, 5) + 5 * z].clone()
                        ^ c[((x + 1) % 5) + 5 * Self::minus_one_mod_z(z, w)].clone(),
                )
            }
        }
        let mut out_bits = Vec::with_capacity(in_bits.len());
        for z in 0..w {
            for y in 0..5 {
                for x in 0..5 {
                    out_bits.push(in_bits[x + y * 5 + z * 25].clone() ^ d[x + z * 5].clone())
                }
            }
        }
        out_bits
    }

    pub fn rho(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.state_length());
        let w = in_bits.len() / 25;
        let mut out_bits = vec![bit!(false); in_bits.len()];
        let rotations = vec![
            0, 1, 190, 28, 91, 36, 300, 6, 55, 276, 3, 10, 171, 153, 231, 105, 45, 15, 21, 136,
            210, 66, 253, 120, 78,
        ];
        for z in 0..w {
            for y in 0..5 {
                for x in 0..5 {
                    out_bits[x + y * 5 + ((z + rotations[x + y * 5]) % w) * 25] =
                        in_bits[x + y * 5 + z * 25].clone();
                }
            }
        }
        out_bits
    }

    pub fn pi(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.state_length());
        let w = in_bits.len() / 25;
        let mut out_bits = Vec::with_capacity(in_bits.len());
        for z in 0..w {
            for y in 0..5 {
                for x in 0..5 {
                    out_bits.push(in_bits[(x + 3 * y) % 5 + 5 * x + 25 * z].clone())
                }
            }
        }
        out_bits
    }

    pub fn chi(&self, in_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(in_bits.len() == self.state_length());
        let w = in_bits.len() / 25;
        let mut out_bits = Vec::with_capacity(in_bits.len());
        for z in 0..w {
            for y in 0..5 {
                let mut shard = Vec::with_capacity(5);
                for x in 0..5 {
                    shard.push(in_bits[x + y * 5 + z * 25].clone())
                }
                out_bits.append(&mut self.chi_sbox.apply(shard))
            }
        }
        out_bits
    }

    pub fn iota(&self, in_bits: Vec<Bit>, round_index: usize) -> Vec<Bit> {
        assert!(in_bits.len() == self.state_length());
        let w = in_bits.len() / 25;
        let l = {
            let tmp = w as f64;
            tmp.log2() as usize
        };
        let mut out_bits = in_bits.clone();
        let mut rc: Vec<Bit> = vec![bit!(false); w];
        for j in 0..=l {
            rc[2usize.pow(j as u32) - 1] = Self::rc_lfsr(j + 7 * round_index);
        }
        for z in 0..w {
            out_bits[z * 25] ^= rc[z].clone();
        }
        out_bits
    }

    fn rc_lfsr(t: usize) -> Bit {
        if t % 255 == 0 {
            bit!(true)
        } else {
            let mut r = vec![
                bit!(true),
                bit!(false),
                bit!(false),
                bit!(false),
                bit!(false),
                bit!(false),
                bit!(false),
                bit!(false),
            ];
            for _i in 1..=(t % 255) {
                let mut vec = vec![bit!(false)];
                vec.append(&mut r);
                r = vec;
                r[0] = r[0].clone() ^ r[8].clone();
                r[4] = r[4].clone() ^ r[8].clone();
                r[5] = r[5].clone() ^ r[8].clone();
                r[6] = r[6].clone() ^ r[8].clone();
                r.truncate(8);
            }

            r[0].clone()
        }
    }

    pub fn add_padding(&self, message_bits: &mut Vec<Bit>) {
        let mut j = self.rate - message_bits.len() % self.rate;
        if j < 2 {
            j += self.rate;
        }
        let mut padding = vec![bit!(true)];
        padding.append(&mut vec![bit!(false); j - 2]);
        padding.push(bit!(true));
        message_bits.append(&mut padding);
    }

    pub fn keccak_permutation(&self, mut in_bits: Vec<Bit>) -> Vec<Bit> {
        for round_index in 0..self.n_rounds {
            in_bits = self.iota(
                self.chi(self.pi(self.rho(self.theta(in_bits)))),
                round_index,
            )
        }
        in_bits
    }
}

impl SpongeHash for Keccak {
    fn hash(&self, mut message_bits: Vec<Bit>) -> Vec<Bit> {
        assert!(message_bits.len() == self.message_length);
        let mut message_shards = Vec::with_capacity(message_bits.len() / self.rate);
        for _i in 0..message_bits.len() / self.rate {
            let tmp = message_bits.split_off(self.rate);
            message_shards.push(message_bits);
            message_bits = tmp;
        }
        let mut state: Vec<Bit> = vec![bit!(false); self.state_length()];
        let w = state.len() / 25;
        for shard in message_shards.iter() {
            'xor: for y in 0..5 {
                for x in 0..5 {
                    for z in 0..w {
                        if z + x * w + y * w * 5 == self.rate {
                            break 'xor;
                        }
                        state[x + y * 5 + z * 25] ^= shard[z + x * w + y * 5 * w].clone();
                    }
                }
            }
            state = self.keccak_permutation(state);
        }
        let mut out_bits = Vec::with_capacity(self.output_length);
        for _i in 0..=(self.output_length / self.rate) {
            'out: for y in 0..5 {
                for x in 0..5 {
                    for z in 0..w {
                        if z + x * w + y * 5 * w == self.rate {
                            break 'out;
                        }
                        out_bits.push(state[x + y * 5 + z * 25].clone());
                    }
                }
            }
            if out_bits.len() < self.output_length {
                state = self.keccak_permutation(state);
            }
        }
        out_bits[..self.output_length].to_vec()
    }

    fn message_length(&self) -> usize {
        self.message_length
    }

    fn state_length(&self) -> usize {
        self.rate + self.capacity
    }

    fn output_length(&self) -> usize {
        self.output_length
    }

    fn rate_length(&self) -> usize {
        self.rate
    }

    fn n_rounds(&self) -> usize {
        self.n_rounds
    }

    fn sbox(&self) -> Sbox {
        self.chi_sbox.clone()
    }
}

pub fn bits_from_hex_string_keccak(h_str: &str) -> Vec<Bit> {
    let h_str = h_str
        .replace("0x", "")
        .replace("0X", "")
        .replace("\\x", "")
        .replace("\\X", "")
        .replace("x", "")
        .replace("X", "")
        .replace(" ", "");
    assert!(h_str.len() % 2 == 0);
    let mut b_str = String::new();
    for i in 0..h_str.len() / 2 {
        b_str.push_str(
            format!(
                "{:08b}",
                u8::from_str_radix(
                    h_str
                        .chars()
                        .skip(i * 2)
                        .take(2)
                        .collect::<String>()
                        .as_str(),
                    16
                )
                .unwrap()
            )
            .chars()
            .rev()
            .collect::<String>()
            .as_str(),
        )
    }
    bit::bits_from_binary_string(&b_str)
}

pub fn bits_to_hex_string_keccak(bits: Vec<Bit>) -> String {
    assert!(bits.len() % 8 == 0);
    let mut hex = String::with_capacity(bits.len() / 4);
    for i in 0..bits.len() / 8 {
        hex.push_str(&format!(
            "{:02x}",
            usize::from_str_radix(
                bits.iter()
                    .skip(i * 8)
                    .take(8)
                    .rev()
                    .map(|bit| if bit.constant() { '1' } else { '0' })
                    .collect::<String>()
                    .as_str(),
                2
            )
            .unwrap()
        ));
    }
    hex
}

// from https://keccak.team/crunchy_contest.html and https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf

#[cfg(test)]
mod test {
    use crate::bit;
    use crate::targets::{
        keccak::{Keccak, *},
        SpongeHash,
    };

    #[test]
    fn test_hex() {
        let expected_bits = bit::bits_from_binary_string("100000000000000000000000000000000000000000011010100110100111000010011010111111011000001001111001100110001000000101101101");
        let bits = bits_from_hex_string_keccak(
            "\\x01\\x00\\x00\\x00\\x00\\x58\\x59\\x0e\\x59\\xbf\\x41\\x9e\\x19\\x81\\xb6",
        );
        assert_eq!(expected_bits, bits);
        let hex = bits_to_hex_string_keccak(bits);
        let expected_hex = "010000000058590e59bf419e1981b6";
        assert_eq!(expected_hex, hex);
    }

    #[test]
    fn test_padding() {
        let mut message_bits: Vec<Bit> = vec![bit!(false); 38];
        let k = Keccak::new(0, 0, 0, 40, 160);
        let mut expected = message_bits.clone();
        expected.append(&mut vec![bit!(true), bit!(true)]);
        k.add_padding(&mut message_bits);
        assert_eq!(expected, message_bits);

        let mut message_bits: Vec<Bit> = vec![bit!(false); 39];
        let k = Keccak::new(0, 0, 0, 40, 160);
        let mut expected = message_bits.clone();
        expected.push(bit!(true));
        expected.append(&mut vec![bit!(false); 39]);
        expected.push(bit!(true));
        k.add_padding(&mut message_bits);
        assert_eq!(expected, message_bits);

        let mut message_bits: Vec<Bit> = vec![bit!(false); 79];
        let k = Keccak::new(0, 0, 0, 40, 160);
        let mut expected = message_bits.clone();
        expected.push(bit!(true));
        expected.append(&mut vec![bit!(false); 39]);
        expected.push(bit!(true));
        k.add_padding(&mut message_bits);
        assert_eq!(expected, message_bits);
    }

    #[test]
    fn validate_hashing() {
        let mut message_bits = bit::bits_from_binary_string("100000000000000000000000000000000000000000011010100110100111000010011010111111011000001001111001100110001000000101101");
        let k = Keccak::new(1, 120, 80, 40, 160);
        k.add_padding(&mut message_bits);
        let hash = k.hash(message_bits);
        let hex_hash = bits_to_hex_string_keccak(hash);
        let expected_hash = "e9f57f02a9b0ebd84498";
        assert_eq!(hex_hash, expected_hash);

        let message_bits = bits_from_hex_string_keccak("\\x11\\xFE\\x35\\xC8\\x5C\\x41\\x5B\\x35\\xF6\\x11\\xBC\\x40\\xD5\\x5E\\xCA\\x16\\xBA\\x51\\x98\\xFA\\x6C\\x42\\xC7\\x08\\x79\\x3A\\x86\\xE9\\xBC\\x50\\x48\\x1F\\xAD\\x98\\xB8\\xCB\\x1B\\x7E\\x87\\xB6\\xA3\\x93\\x59\\x24\\xDB\\x03\\xB0\\xEB\\x23\\xB0\\x97\\xD0\\x87\\xA4\\x7C\\xF0\\x14\\x61\\x3A\\x43\\xF4\\x3B\\x97\\x43\\xBA\\x4B\\x5D\\x04\\xAA\\xBD\\xC5\\x22\\xB5\\x66\\x59\\x9B\\x2C\\x5E\\xF8\\x1A\\xB3\\xBC\\x8C\\x2F\\x21\\x89\\xC0\\xAC\\x33\\xE7\\x38\\xAB\\x4B\\x99\\x18\\xA4\\x0B\\x02\\x4C\\xF0\\x69\\xA3\\xED\\xD5\\x17\\xA1\\xEB\\x7F\\x87\\x61\\xC9\\x5C\\x23\\xC6\\x6B\\x08\\x88\\xE9\\x86\\x94\\x67\\x75\\x0D\\x0B\\x4D\\xD6\\x13\\xAC\\xA1\\x92\\x6A\\x89\\xF5\\xAD\\x8B\\x57\\x87\\xD8\\x6E\\x4F\\xDC\\xD0\\x2B\\x28\\x2A\\x93\\x1E\\xE8\\x10\\xB6\\xAB\\xF5\\x36\\x34\\xB7\\x11\\x6D\\xDF\\xCA\\x1A\\x88\\x83\\xBA\\x57\\x61\\xE3\\xC9\\x5E\\x38\\x63\\xC0\\x04\\x6F\\x43\\x68\\xCA\\x0A\\xA0\\xAE\\x9A");
        let k = Keccak::new(2, 1440, 80, 1440, 160);
        let hash = k.hash(message_bits);
        let hex_hash = bits_to_hex_string_keccak(hash);
        let expected_hash = "6390220e7b5d3284d23e";
        assert_eq!(hex_hash, expected_hash);
    }

    #[test]
    fn validate_collision() {
        let message_bits = bits_from_hex_string_keccak("\\x3f\\xb7\\x7d\\x29\\x6d\\xb4\\x5f\\xce\\xab\\xd5\\xef\\x63\\xb2\\xdb\\x75\\xab\\xe7\\x19\\x01\\x02\\x73\\x77\\x92\\x06\\xa4\\xa6\\x45\\xa6\\xf8\\xe3\\xe6\\x68\\x62\\x24\\x28\\x15\\x83\\xab\\x3a\\x63\\xfb\\xa5\\xc7\\x96\\xb5\\xbe\\x4c\\x5e\\x96\\x4c\\x61\\x92\\xda\\x47\\x96\\xdd\\x4f\\x09\\xb0\\xd4\\x6f\\x37\\x68\\x4c\\x51\\x37\\xb6\\xd3\\x56\\xab\\x86\\x62\\x52\\x7a\\x57\\xde\\x0f\\xea\\x03\\x90");
        let k = Keccak::new(5, 640, 160, 640, 160);
        let hash = k.hash(message_bits);
        let hex_hash = bits_to_hex_string_keccak(hash);
        let expected_hash = "ba5a0bf92d683074628c6685adb0e16635ac52b0";
        assert_eq!(hex_hash, expected_hash);
        let message_bits = bits_from_hex_string_keccak("\\xcb\\x0b\\x15\\x5b\\xfc\\xf2\\xf3\\xc0\\xa5\\xb5\\x07\\x59\\xc3\\x6d\\x73\\x22\\xc5\\xf0\\x4c\\x91\\x63\\x7d\\x47\\x87\\x49\\xa6\\x75\\xa6\\x6f\\xa9\\xbe\\x8b\\xe3\\x8a\\xeb\\x52\\x41\\x2d\\x40\\x19\\xc3\\x4c\\xfb\\xd9\\x30\\xd6\\x9f\\x66\\x71\\xfc\\xc9\\xd8\\x54\\x85\\x55\\x57\\x4a\\xf6\\x62\\x06\\xc5\\xb5\\xb4\\x64\\x56\\xbf\\x12\\x7f\\xf0\\xdb\\xea\\x2b\\x10\\x7b\\x20\\xf6\\x87\\x97\\xfd\\xf2");
        let k = Keccak::new(5, 640, 160, 640, 160);
        let hash = k.hash(message_bits);
        let hex_hash = bits_to_hex_string_keccak(hash);
        let expected_hash = "ba5a0bf92d683074628c6685adb0e16635ac52b0";
        assert_eq!(hex_hash, expected_hash);
    }
}
