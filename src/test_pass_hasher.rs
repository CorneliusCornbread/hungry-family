/// Small helper binary: cargo run --bin hash_password -- "my_password"
/// Prints an argon2id hash suitable for inserting into the accounts table.
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

fn main() {
    let password = std::env::args()
        .nth(1)
        .expect("Usage: hash_password <password>");

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    println!("{hash}");
}
