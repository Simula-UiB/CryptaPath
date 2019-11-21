#[macro_use]
extern crate crush;
extern crate rand;
extern crate structopt;
extern crate vob;
#[macro_use]
extern crate structopt_derive;

#[macro_use]
pub mod bit;
pub mod options;
pub mod sbox;
pub mod strategy;
pub mod targets;

use crush::soc::utils::*;
use options::CryptaPathOptions;
use structopt::StructOpt;
use targets::*;

fn main() {
    match CryptaPathOptions::from_args() {
        CryptaPathOptions::Cipher {
            cipher_name,
            rounds,
            chosen_plaintext_ciphertext,
            key,
            out,
            strategy,
        } => {
            let cipher = match build_cipher_by_name(cipher_name.as_ref(), rounds) {
                Some(c) => c,
                None => {
                    println!("Cipher not supported. Check --help for supported ciphers.");
                    return;
                }
            };
            let (input, output, mut system) = build_system_cipher(cipher.as_ref());
            let (plaintext, ciphertext);
            if let Some(plaintext_ciphertext) = chosen_plaintext_ciphertext {
                assert_eq!(
                    plaintext_ciphertext.len(),
                    2,
                    "You can only provide one plaintext and one ciphertext"
                );
                plaintext = bit::bits_from_hex_string(&plaintext_ciphertext[0]);
                ciphertext = bit::bits_from_hex_string(&plaintext_ciphertext[1]);
                if let Some(partial_key) = key {
                    let filled_key = fill_partial_value(partial_key.as_ref());
                    assert_eq!(cipher.key_length(), filled_key.0.len(),
                    "the provided partial key has a size different from the key expected by the chosen cipher");
                    fix_system_values_cipher_with_partial_key(
                        &mut system,
                        &plaintext,
                        &ciphertext,
                        filled_key,
                        &input,
                        &output,
                    );
                } else {
                    fix_system_values_cipher(&mut system, &plaintext, &ciphertext, &input, &output);
                }
            } else if let Some(partial_key) = key {
                let filled_key = fill_partial_value(partial_key.as_ref());
                let tmp = get_random_plaintext_ciphertext_with_partial_key(
                    cipher.as_ref(),
                    filled_key.0.clone(),
                );
                plaintext = tmp.0;
                ciphertext = tmp.1;
                fix_system_values_cipher_with_partial_key(
                    &mut system,
                    &plaintext,
                    &ciphertext,
                    filled_key,
                    &input,
                    &output,
                );
            } else {
                let tmp = get_random_plaintext_ciphertext_key(cipher.as_ref());
                plaintext = tmp.0;
                ciphertext = tmp.1;
                fix_system_values_cipher(&mut system, &plaintext, &ciphertext, &input, &output);
            }
            if let Some(path) = out {
                print_system_to_file(&system, &path);
            }
            let forbid_dropping: Vec<usize> = (0..cipher.key_length()).collect();
            let mut sols = match strategy {
                Some(name) => match strategy::execute_strategy_by_name(
                        name.as_ref(),
                        &mut system,
                        Some(&forbid_dropping),
                    ) {
                        Some(sols) => sols,
                        None => {
                            println!("Strategy not supported. Check --help for supported strategies.");
                            return;
                        }
                    }
                ,
                None => {
                    strategy::execute_strategy_by_name("no_drop", &mut system, None).unwrap()
                }
            };
            for sol in sols.iter_mut() {
                sol.split_off(cipher.key_length());
                let mut binary_string_sol = String::new();
                for var in sol.iter() {
                    match var {
                        Some(b) => match b {
                            true => {
                                binary_string_sol.push('1');
                            }
                            false => {
                                binary_string_sol.push('0');
                            }
                        },
                        None => {
                            if cipher_name == "des" {
                                // with des this will always be the case as some bits of the 64 bit key are
                                // unused. We can therefore just push 0 and the encryption will validate.
                                // Kind of an ugly fix, the better fix would be to limit des to 56 bits and change
                                // the test vectors
                                binary_string_sol.push('0')
                            } else {
                                panic!("Some bits of the key are not determined, something wrong happened during the solving")
                            }
                        }
                    }
                }
                let key = bit::bits_from_binary_string(&binary_string_sol);
                assert_eq!(
                    ciphertext,
                    cipher.encrypt(plaintext.clone(), key.clone()),
                    "A solution was found but it doesn't encrypt correctly, something went wrong"
                );
                println!("valid solution : {}", bit::bits_to_hex_string(key));
            }
        }

        CryptaPathOptions::Sponge {
            sponge,
            rounds,
            message_length,
            hash_length,
            rate,
            capacity,
            image,
            preimage,
            out,
        } => {
            assert_eq!(
                message_length % rate,
                0,
                "message_length should be a multiple of rate"
            );
            let hash = match build_sponge_by_name(
                sponge.as_ref(),
                rounds,
                message_length,
                hash_length,
                rate,
                capacity,
            ) {
                Some(h) => h,
                None => {
                    println!("Sponge not supported. Check --help for supported sponges.");
                    return;
                }
            };
            let (output, mut system) = build_system_sponge(hash.as_ref());
            let preimage_filled = match preimage {
                Some(pre) => {
                    assert!(pre.ends_with('1'),
                    "the last bit of preimage has to be a 1 (padding is included in the preimage provided)");
                    Some(fill_partial_value(pre.as_ref()))
                }
                None => None,
            };
            let hash_value = match image {
                None => match preimage_filled.clone() {
                    None => get_random_sponge_output(hash.as_ref()),
                    Some(p) => get_sponge_output_with_partial_preimage(hash.as_ref(), p.0),
                },
                Some(image) => keccak::bits_from_hex_string_keccak(image.as_ref()),
            };
            match preimage_filled {
                Some(p) => fix_system_values_sponge_with_partial_preimage(
                    hash.as_ref(),
                    &mut system,
                    &hash_value,
                    &output,
                    p,
                ),
                None => fix_system_values_sponge(hash.as_ref(), &mut system, &hash_value, &output),
            }
            if let Some(path) = out {
                print_system_to_file(&system, &path);
            }
            let forbid_dropping: Vec<usize> = (0..hash.message_length()).collect();
            let mut sols = strategy::execute_strategy_by_name(
                "UpwardDroppingSolver",
                &mut system,
                Some(&forbid_dropping),
            )
            .unwrap();
            for sol in sols.iter_mut() {
                sol.split_off(hash.message_length());
                let mut binary_string_sol = String::new();
                for var in sol.iter() {
                    match var {
                        Some(b) => match b {
                            true => {
                                binary_string_sol.push('1');
                            }
                            false => {
                                binary_string_sol.push('0');
                            }
                        },
                        None => panic!("shouldn't happen"),
                    }
                }
                let preimage = bit::bits_from_binary_string(&binary_string_sol);
                assert_eq!(hash_value, hash.hash(preimage.clone()));
                println!(
                    "valid solution : {}",
                    keccak::bits_to_hex_string_keccak(preimage)
                );
            }
        }

        CryptaPathOptions::MakeParam { cipher, rounds } => {
            let cipher = match build_cipher_by_name(cipher.as_ref(), rounds) {
                Some(c) => c,
                None => {
                    println!("Cipher not supported. Check --help for supported ciphers.");
                    return;
                }
            };
            let (plaintext, ciphertext, key) = get_random_plaintext_ciphertext_key(cipher.as_ref());
            println!("plaintext : {}", bit::bits_to_hex_string(plaintext));
            println!("ciphertext : {}", bit::bits_to_hex_string(ciphertext));
            println!("key : {}", bit::bits_to_binary_string(key));
        }
        CryptaPathOptions::FromFile { file } => {
            let specs = parse_system_spec_from_file(&file);
            let mut system = build_system_from_spec(specs);
            strategy::execute_strategy_by_name("UpwardSolver", &mut system, None).unwrap();
        }
    }
}
