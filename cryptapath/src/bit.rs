//! An implementation of a `Bit` used for implementing S-Boxes and cryptosystem.
//! A `Bit` can hold a set of variable (kept in a BtreeSet) and a constant (a bool).
//! It can be used interchangeably to represent a constant bit (the set will be empty)
//! or a linear combination of variable (sum of variable present in the set + the
//! value of the constant 0 or 1).
//! 
//! A `Bit` can be XORed with another `Bit` by using the implementation of `BitXor` or
//! `BitXorAssign`, and, because this is an operation used in almost all cryptosystems,
//! a `Vec<Bit>` can be XORed with another `Vec<Bit>` by using the function 
//! `bit_vector_xoring`. XORing 2 Bit result in a Bit constaining the symetric difference
//! of variables and a constant equal to the xor operation between the two constants.
//! The AND function is not implemented because we don't support multiplying variables
//!  in our use case.
//! 
//! Are also provided easy convertion from hexadecimal and binary string to Vec of constant
//! bits and vice versa, a function to generate Vec of random constant bits and the 
//! bit! macro to create a single constant bit.


use crate::rand::distributions::{Distribution, Uniform};
use std::collections::{btree_set::Iter,BTreeSet};
use std::fmt;
use std::ops::{BitXor, BitXorAssign};

/// A wrapper around usize, a single variable in a system.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub struct Variable {
    id: usize,
}

impl Variable {
    /// Returns a new variable of value id.
    #[inline]
    pub fn new(id: usize) -> Self {
        Variable { id }
    }

    /// Return the id of the variable.
    #[inline]
    pub fn id(self) -> usize {
        self.id
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// Return a constant bit of value true or false.
#[macro_export]
macro_rules! bit {
    ($value:expr) => {
        $crate::bit::Bit::from_value($value)
    };
}

/// A Bit in a cryptosystem (see module documentation for more information)
#[derive(Default, Debug, Eq, PartialEq, PartialOrd, Ord, Clone)]
pub struct Bit {
    pub vars: BTreeSet<Variable>,
    constant: bool,
}

impl Bit { 
    /// Return a new bit (equivalent to bit!(false)).
    pub fn new() -> Self {
        Default::default()
    }

    /// Return a constant bit (equivalent to the bit! macro).
    pub fn from_value(constant: bool) -> Self {
        Bit {
            vars: BTreeSet::new(),
            constant,
        }
    }
    
    /// Return a bit of constant value false and containing one variable of var_id.
    pub fn from_variable_id(var_id: usize) -> Self {
        let mut vars = BTreeSet::new();
        vars.insert(Variable::new(var_id));
        Bit {
            vars,
            constant: false,
        }
    }

    /// Return the constant part of a Bit.
    #[inline]
    pub fn constant(&self) -> bool {
        self.constant
    }

    /// Return an iterator over the variables in the Bit.
    pub fn vars(&self) -> Iter<Variable> {
        self.vars.iter()
    }
}

/// Convert a binary string (ie a string composed of '0' and '1') to the corresponding Vec<Bit>
/// with all Bit in the Vec constants.
pub fn bits_from_binary_string(b_str: &str) -> Vec<Bit> {
    let out_bits: Vec<Bit> = b_str
        .chars()
        .map(|char| match char {
            '0' => bit!(false),
            '1' => bit!(true),
            _ => panic!(format!("{} this is not a binary string", b_str)),
        })
        .collect();
    out_bits
}

/// Convert an hex string (ie a string composed of hexadecimal characters) to the corresponding Vec<Bit>
/// with all Bit in the Vec constants.
pub fn bits_from_hex_string(h_str: &str) -> Vec<Bit> {
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
            .collect::<String>()
            .as_str(),
        )
    }
    bits_from_binary_string(&b_str)
}

/// Convert a Vec<Bit> to an hexadecimal string by taking the value
/// of the constants. If some variable are in the bit they will be ignored.
pub fn bits_to_hex_string(bits: Vec<Bit>) -> String {
    assert!(bits.len() % 8 == 0);
    let mut hex = String::with_capacity(bits.len() / 4);
    for i in 0..bits.len() / 8 {
        hex.push_str(&format!(
            "{:02x}",
            usize::from_str_radix(
                bits.iter()
                    .skip(i * 8)
                    .take(8)
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

/// Convert a Vec<Bit> to a binary string by taking the value
/// of the constants. If some variable are in the bit they will be ignored.
pub fn bits_to_binary_string(bits: Vec<Bit>) -> String {
    bits.iter()
        .map(|bit| if bit.constant() { '1' } else { '0' })
        .collect::<String>()
}

/// Produce a Vec<Bit> of the provided len with constants random bits.
pub fn random_bits(len: usize) -> Vec<Bit> {
    let mut rng = rand::thread_rng();
    let die = Uniform::from(0..2);
    let mut bits = Vec::with_capacity(len);
    for _ in 0..len {
        let throw = die.sample(&mut rng);
        match throw {
            0 => bits.push(bit!(false)),
            1 => bits.push(bit!(true)),
            _ => panic!("not supposed to happen"),
        }
    }
    bits
}

/// Return a new Vec<Bit> produced by XORing each bits of the two vectors.
/// The two vectors must contains the same number of bits.
pub fn bit_vector_xoring(mut a: Vec<Bit>, mut b: Vec<Bit>) -> Vec<Bit> {
    assert_eq!(a.len(), b.len());
    a.drain(..)
        .zip(b.drain(..))
        .map(|(a_bit, b_bit)| a_bit ^ b_bit)
        .collect::<Vec<Bit>>()
}

impl BitXor for Bit {
    type Output = Self;

    fn bitxor(self, rhs: Bit) -> Self::Output {
        Bit {
            vars: self
                .vars
                .symmetric_difference(&rhs.vars)
                .copied().collect(),
            constant: self.constant ^ rhs.constant,
        }
    }
}

impl BitXorAssign for Bit {
    fn bitxor_assign(&mut self, rhs: Bit) {
        self.vars = self
            .vars
            .symmetric_difference(&rhs.vars)
            .copied()
            .collect();
        self.constant ^= rhs.constant;
    }
}

#[test]
fn test_xor() {
    let vars = vec![
        Variable::new(0),
        Variable::new(1),
        Variable::new(2),
        Variable::new(3),
    ];
    let mut bit_1 = Bit::new();
    bit_1.vars.insert(vars[0]);
    bit_1.vars.insert(vars[1]);
    bit_1.vars.insert(vars[2]);
    let mut bit_2 = Bit::new();
    bit_2.vars.insert(vars[0]);
    bit_2.vars.insert(vars[2]);
    bit_2.vars.insert(vars[3]);
    bit_2.constant = true;
    let mut bit_3 = Bit::new();
    bit_3.vars.insert(vars[1]);
    bit_3.vars.insert(vars[3]);
    bit_3.constant = true;
    assert_eq!(bit_3, bit_1 ^ bit_2);
}
