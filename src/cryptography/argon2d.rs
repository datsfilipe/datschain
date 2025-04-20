use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};

pub fn hash(value: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let algo = Algorithm::new("argon2d").unwrap();
    let argon2 = Argon2::new(algo, Version::default(), Params::default());
    let result = argon2.hash_password(value.as_bytes(), &salt);
    match result {
        Ok(hash) => Ok(format!("{}", hash)),
        Err(err) => Err(format!("{}", err)),
    }
}
