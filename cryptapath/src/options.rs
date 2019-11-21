use std::path::PathBuf;

#[derive(Clone, StructOpt)]
#[structopt(
    name = "CryptaPath",
    about = "A tool to generate systems of BDD from an implementation and solve it",
    author = "SimulaUiB"
)]
pub enum CryptaPathOptions {
    #[structopt(name = "cipher")]
    Cipher {
        #[structopt(short = "c", long = "cipher")]
        ///Name of the target cipher. Currently supported: 
        ///skinny64128, skinny128128, lowmc64, lowmc128, lowmc256, miniaes2x2, miniaes4x4, present80, prince, prince-core, des
        cipher_name: String,
        #[structopt(short = "r", long = "rounds")]
        ///The number of rounds to run on the cipher
        rounds: usize,
        #[structopt(short = "p", long = "plaintext_ciphertext")]
        /// A pair of plaintext/ciphertext encrypted under a valid key by the target cipher
        /// The expected format is hexadecimal.
        /// Make sure the pair is compatible with the key provided (if you decide to provide one)
        /// or you'll encounter a "this system has no solution" error when trying to solve.
        /// If not provided a random pair will be generate by generating a random plaintext and encrypting
        /// it under a key.
        chosen_plaintext_ciphertext: Option<Vec<String>>,
        #[structopt(short = "k", long = "key")]
        ///If provided, this indicate the known bits of the key.
        ///The String should contain only X or x for the unknown bits and 0 or 1 for the known bits,
        ///and be the exact length of the key used by the chosen cipher.
        ///For example: 000X100X0111XXXXX111100000XXXXXX is a valid 32 bits key.
        ///The value of the missing X will be fill randomly at run time to produce a pair of plaintext/ciphertext
        ///if none are provided.
        ///If not provided a completely random key will be generated at run time (equivalent to all X).
        key: Option<String>,
        #[structopt(short = "o", long = "output", parse(from_os_str))]
        /// If provided will output a .bdd file of the system (after fixing the values) at the provided path
        out: Option<PathBuf>,
        #[structopt(short = "s", long = "strategy")]
        /// Choose the strategy when trying to solve.
        /// Available choices: "drop" "no_drop", default: "no_drop"
        strategy: Option<String>,
    },
    #[structopt(name = "sponge")]
    Sponge {
        #[structopt(short = "s", long = "sponge")]
        ///Name of the target SpongeHash. Currently supported: keccak
        sponge: String,
        #[structopt(short = "r", long = "rounds")]
        ///The number of rounds to run on the hash
        rounds: usize,
        #[structopt(long = "message-length")]
        /// The length of your message (including padding), should be a multiple of
        /// the rate of your instance.
        message_length: usize,
        #[structopt(long = "hash-length")]
        /// The length of the hash produced by the squeeze part
        hash_length: usize,
        #[structopt(long = "rate")]
        /// The size of the rate part of the state
        rate: usize,
        #[structopt(long = "capacity")]
        /// The size of the capacity part of the state
        capacity: usize,
        #[structopt(long = "image")]
        /// If provided, the image for which we will try to find preimages
        /// The image should be provided in hexadecimal, will use the
        /// hexadecimal conversion from FIPS 202 and should be equal to the
        /// hash-length specified
        image: Option<String>,
        #[structopt(long = "partial-preimage")]
        /// If provided, gives information about the preimage.
        /// The image should be provided in binary format where
        /// 0 and 1 is a fixed value and X (or x) indicate an unknown bit.
        /// If no image where provided the partial preimage will be fill
        /// with random value on unknown bits and hashed to create an image.
        preimage: Option<String>,
        #[structopt(short = "o", long = "output", parse(from_os_str))]
        /// If provided will output a .bdd file of the system (after fixing the values) at the provided path
        out: Option<PathBuf>
    },

    #[structopt(name = "make-cipher-param")]
    MakeParam {
        #[structopt(short = "c", long = "cipher")]
        ///Name of the target cipher. Currently supported: 
        ///skinny64128, skinny128128, lowmc64, lowmc128, lowmc256, miniaes2x2, miniaes4x4, present80, prince, prince-core, des
        cipher: String,
        #[structopt(short = "r", long = "rounds")]
        ///The number of rounds to run on the cipher
        rounds: usize,
    },
    #[structopt(name = "from-file")]
    FromFile {
        #[structopt(short = "f", long = "file", parse(from_os_str))]
        /// The source bdd file
        file: PathBuf
    }
}