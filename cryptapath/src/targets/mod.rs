pub mod des;
pub mod keccak;
pub mod lowmc;
pub mod miniaes2x2;
pub mod miniaes4x4;
pub mod present80;
pub mod prince;
pub mod skinny128;
pub mod skinny64;

use des::DES;
use keccak::Keccak;
use lowmc::LowMC;
use miniaes2x2::MiniAES2x2;
use miniaes4x4::MiniAES4x4;
use present80::Present80;
use prince::Prince;
use skinny128::Skinny128;
use skinny64::Skinny64;

use crate::bit::{self, Bit, *};
use crate::sbox::Sbox;
use crush::soc::{
    system::System,
    utils::{SystemSpec, *},
};

pub trait SpongeHash {
    fn hash(&self, in_bits: Vec<Bit>) -> Vec<Bit>;
    fn message_length(&self) -> usize;
    fn rate_length(&self) -> usize;
    fn state_length(&self) -> usize;
    fn output_length(&self) -> usize;
    fn n_rounds(&self) -> usize;
    fn sbox(&self) -> Sbox;
}

pub trait Cipher {
    fn encrypt(&self, in_bits: Vec<Bit>, key_bits: Vec<Bit>) -> Vec<Bit>;
    fn message_length(&self) -> usize;
    fn n_rounds(&self) -> usize;
    fn key_length(&self) -> usize;
    fn sbox(&self) -> Sbox;
}

pub fn build_system_sponge(hash: &dyn SpongeHash) -> (Vec<Bit>, System) {
    let mut message_bits = Vec::with_capacity(hash.message_length());
    for i in 0..hash.message_length() {
        message_bits.push(Bit::from_variable_id(i));
    }
    let output = hash.hash(message_bits);
    let mut sbox = hash.sbox();
    let bdds = sbox.bdds();
    let mut n_state = hash.message_length() / hash.rate_length();
    if hash.message_length() % hash.rate_length() > 0 {
        n_state += 1
    }
    n_state += hash.output_length() / hash.rate_length();
    if hash.output_length() % hash.rate_length() > 0 {
        n_state += 1
    }
    let system_spec = SystemSpec::new(
        hash.message_length() + (hash.state_length() * hash.n_rounds()) * n_state,
        bdds,
    );
    (output, build_system_from_spec(system_spec))
}

pub fn build_system_cipher(cipher: &dyn Cipher) -> (Vec<Bit>, Vec<Bit>, System) {
    let mut message_bits = Vec::with_capacity(cipher.message_length());
    let mut key_bits = Vec::with_capacity(cipher.key_length());
    for i in 0..cipher.key_length() {
        key_bits.push(Bit::from_variable_id(i));
    }
    for i in cipher.key_length()..cipher.message_length() + cipher.key_length() {
        message_bits.push(Bit::from_variable_id(i));
    }
    let output = cipher.encrypt(message_bits.clone(), key_bits);
    let mut sbox = cipher.sbox();
    let bdds = sbox.bdds();
    let system_spec = SystemSpec::new(sbox.next_var_id(), bdds);
    (message_bits, output, build_system_from_spec(system_spec))
}

pub fn get_random_sponge_output(hash: &dyn SpongeHash) -> (Vec<Bit>) {
    let random_preimage = random_bits(hash.message_length());
    hash.hash(random_preimage)
}

pub fn get_random_plaintext_ciphertext_key(cipher: &dyn Cipher) -> (Vec<Bit>, Vec<Bit>,Vec<Bit>) {
    let random_plaintext = random_bits(cipher.message_length());
    let random_key = random_bits(cipher.key_length());
    (
        random_plaintext.clone(),
        cipher.encrypt(random_plaintext, random_key.clone()),
        random_key
    )
}

pub fn fill_partial_value(partial_value: &str) -> (Vec<Bit>, Vec<usize>) {
    let mut known_bits = Vec::new();
    let mut value = Vec::with_capacity(partial_value.len());
    partial_value
        .chars()
        .enumerate()
        .for_each(|(i, c)| match c {
            'x' | 'X' => value.push(bit::random_bits(1).pop().unwrap()),
            '0' => {
                value.push(bit!(false));
                known_bits.push(i)
            }
            '1' => {
                value.push(bit!(true));
                known_bits.push(i)
            }
            _ => panic!("illegal char in value string, should only contain X, x, 0 or 1"),
        });
    (value, known_bits)
}

pub fn get_random_plaintext_ciphertext_with_partial_key(
    cipher: &dyn Cipher,
    partial_key: Vec<Bit>,
) -> (Vec<Bit>, Vec<Bit>) {
    assert_eq!(
        cipher.key_length(),
        partial_key.len(),
        "the provided partial key has a size different from the key expected by the chosen cipher"
    );

    let random_plaintext = random_bits(cipher.message_length());
    (
        random_plaintext.clone(),
        cipher.encrypt(random_plaintext, partial_key),
    )
}

pub fn get_sponge_output_with_partial_preimage(
    hash: &dyn SpongeHash,
    partial_preimage: Vec<Bit>,
) -> (Vec<Bit>) {
    assert_eq!(
        hash.message_length(),
        partial_preimage.len(),
        "the provided partial preimage has a size different from the preimage expected by the chosen sponge"
    );
    hash.hash(partial_preimage)
}

pub fn fix_system_values_sponge(
    hash: &dyn SpongeHash,
    system: &mut System,
    hash_value: &[Bit],
    output_bits: &[Bit],
) {
    let padding_bit = {
        if hash.message_length() <= hash.rate_length() {
            hash.rate_length() - 1
        } else {
            hash.message_length() + hash.message_length() % hash.rate_length() - 1
        }
    };
    //fixing padding (every padding end with a one regardless of the message_length)
    system.fix(vec![padding_bit], true).unwrap();
    //fixing the value of the output
    for (output_bit, expected_bit) in output_bits.iter().zip(hash_value) {
        system
            .fix(
                output_bit.vars.iter().map(|var| var.id()).collect(),
                output_bit.constant() ^ expected_bit.constant(),
            )
            .unwrap();
    }
}

pub fn fix_system_values_sponge_with_partial_preimage(
    hash: &dyn SpongeHash,
    system: &mut System,
    hash_value: &[Bit],
    output_bits: &[Bit],
    mut partial_preimage: (Vec<Bit>,Vec<usize>)
) {
    fix_system_values_sponge(hash, system, hash_value, output_bits);
    let padding_bit = {
        if hash.message_length() <= hash.rate_length() {
            hash.rate_length() - 1
        } else {
            hash.message_length() + hash.message_length() % hash.rate_length() - 1
        }
    };
    // We already fixed the padding bit, so if the last bit of the preimage
    // is known (and it has to be a 1 then we skip it)
    let last_known_bit = *partial_preimage.1.iter().last().unwrap();
    if last_known_bit == padding_bit {
        partial_preimage.1.pop();
        partial_preimage.0.pop();
    }
    //fixing the known bits of the preimage
    for known_bit in partial_preimage.1.iter() {
        system
            .fix(vec![*known_bit], partial_preimage.0[*known_bit].constant())
            .unwrap();
    }
}

pub fn fix_system_values_cipher(
    system: &mut System,
    plaintext: &[Bit],
    ciphertext: &[Bit],
    input_bits: &[Bit],
    output_bits: &[Bit],
) {
    for (plaintext_vars, plaintext_bits) in input_bits.iter().zip(plaintext) {
        system
            .fix(
                plaintext_vars.vars.iter().map(|var| var.id()).collect(),
                plaintext_vars.constant() ^ plaintext_bits.constant(),
            )
            .unwrap();
    }
    for (ciphertext_vars, expected_bit) in output_bits.iter().zip(ciphertext) {
        system
            .fix(
                ciphertext_vars.vars.iter().map(|var| var.id()).collect(),
                ciphertext_vars.constant() ^ expected_bit.constant(),
            )
            .unwrap();
    }
}

pub fn fix_system_values_cipher_with_partial_key(
    system: &mut System,
    plaintext: &[Bit],
    ciphertext: &[Bit],
    partial_key: (Vec<Bit>, Vec<usize>),
    input_bits: &[Bit],
    output_bits: &[Bit],
) {
    // This assumes that the key variables are always the n first (from 1 to key_length)
    // In pratice this is safe because we use this assumption everywhere but in case
    // someone would like to tinker with the library this has to be taken into account.
    for known_bit in partial_key.1.iter() {
        system
            .fix(vec![*known_bit], partial_key.0[*known_bit].constant())
            .unwrap();
    }
    fix_system_values_cipher(system, plaintext, ciphertext, input_bits, output_bits);
}

pub fn build_sponge_by_name(
    name: &str,
    n_rounds: usize,
    message_length: usize,
    output_length: usize,
    rate: usize,
    capacity: usize,
) -> Option<Box<dyn SpongeHash>> {
    match name {
        "keccak" => Some(Box::new(Keccak::new(
            n_rounds,
            message_length,
            output_length,
            rate,
            capacity,
        ))),
        _ => None,
    }
}

pub fn build_cipher_by_name(name: &str, rounds: usize) -> Option<Box<dyn Cipher>> {
    match name {
        "skinny64128" => Some(Box::new(Skinny64::new(128, rounds))),
        "skinny128128" => Some(Box::new(Skinny128::new(128, rounds))),
        "lowmc64" => Some(Box::new(LowMC::new(rounds, 64, 80, 1))),
        "lowmc128" => Some(Box::new(LowMC::new(rounds, 128, 80, 31))),
        "lowmc256" => Some(Box::new(LowMC::new(rounds, 256, 256, 1))),
        "miniaes2x2" => Some(Box::new(MiniAES2x2::new(rounds))),
        "miniaes4x4" => Some(Box::new(MiniAES4x4::new(rounds))),
        "present80" => Some(Box::new(Present80::new(rounds))),
        "prince" => Some(Box::new(Prince::new(rounds, true))),
        "prince-core" => Some(Box::new(Prince::new(rounds, false))),
        "des" => Some(Box::new(DES::new(rounds))),
        _ => None,
    }
}
