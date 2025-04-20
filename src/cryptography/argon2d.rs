use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2,
};

pub fn hash(value: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let result = Argon2::<Algorithm::Argon2d>::default().hash_password(value, &salt);
    match result {
        Ok(hash) => Ok(format!("{}", hash)),
        Err(err) => Err(format!("{}", err)),
    }
}

pub fn verify(value: &str, hash: &str) -> Result<String, String> {
    let result = Argon2::<Algorithm::Argon2d>::default().verify_password(value, hash);
    match result {
        Ok(hash) => Ok(format!("{}", hash)),
        Err(err) => Err(format!("{}", err)),
    }
}
