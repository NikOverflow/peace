use super::{EmailError, PasswordError, UsernameError};
use argon2::Config;
use once_cell::sync::OnceCell;
use rand::Rng;
use regex::Regex;
use std::{marker::PhantomData, ops::Deref};

const PASSWORD_SALT_RNG_LEN: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct CreateUser {
    pub name: Username<Ascii>,
    pub name_unicode: Option<Username<Unicode>>,
    pub password: Password,
    pub email: Email,
    pub country: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Email(String);

impl Email {
    pub fn regex() -> &'static Regex {
        static EMAIL_REGEX: OnceCell<Regex> = OnceCell::new();
        EMAIL_REGEX.get_or_init(|| {
            Regex::new(r"^[^@\s]{1,200}@[^@\s\.]{1,30}\.[^@\.\s]{2,24}$")
                .unwrap()
        })
    }

    pub fn from_str(s: &str) -> Result<Self, EmailError> {
        let s = s.trim().to_ascii_lowercase();

        if !Self::regex().is_match(s.as_str()) {
            return Err(EmailError)
        }

        Ok(Self(s))
    }
}

impl Into<String> for Email {
    #[inline]
    fn into(self) -> String {
        self.0
    }
}

impl From<String> for Email {
    #[inline]
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Deref for Email {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<String> for Email {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

pub trait Checker {
    fn check(s: &str) -> Result<(), UsernameError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ascii;

impl Checker for Ascii {
    #[inline]
    fn check(s: &str) -> Result<(), UsernameError> {
        if !s.is_ascii() {
            return Err(UsernameError)
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unicode;

impl Checker for Unicode {
    #[inline]
    fn check(_: &str) -> Result<(), UsernameError> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsernameSafe(String);

impl<T> Into<Username<T>> for UsernameSafe
where
    T: Checker,
{
    #[inline]
    fn into(self) -> Username<T> {
        Username::new(self.0)
    }
}

impl Into<String> for UsernameSafe {
    #[inline]
    fn into(self) -> String {
        self.0
    }
}

impl From<String> for UsernameSafe {
    #[inline]
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Deref for UsernameSafe {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<String> for UsernameSafe {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Username<T>(String, PhantomData<T>);

impl<T> Username<T>
where
    T: Checker,
{
    #[inline]
    fn new(s: String) -> Self {
        Self(s, PhantomData)
    }

    #[inline]
    pub fn from_str(s: &str) -> Result<Self, UsernameError> {
        T::check(s)?;
        Ok(Self::new(s.trim().to_owned()))
    }

    #[inline]
    pub fn safe_name(&self) -> UsernameSafe {
        UsernameSafe(self.0.to_ascii_lowercase().replace(' ', "_"))
    }
}

impl<T> Into<String> for Username<T> {
    #[inline]
    fn into(self) -> String {
        self.0
    }
}

impl<T> From<String> for Username<T> {
    #[inline]
    fn from(s: String) -> Self {
        Self(s, PhantomData)
    }
}

impl<T> Deref for Username<T> {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<String> for Username<T> {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

pub struct PasswordSalt;

impl PasswordSalt {
    #[inline]
    pub fn salt() -> &'static String {
        static PASSWORD_SALT: OnceCell<String> = OnceCell::new();
        PASSWORD_SALT.get_or_init(|| Self::generate(PASSWORD_SALT_RNG_LEN))
    }

    #[inline]
    pub fn generate(len: usize) -> String {
        rand::thread_rng()
            .sample_iter(rand::distributions::Alphanumeric)
            .take(len)
            .map(char::from)
            .collect::<String>()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Password(String);

impl Password {
    #[inline]
    pub fn from_hashed(hashed_password: String) -> Password {
        Password(hashed_password)
    }

    #[inline]
    pub fn hash(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn hash_password<T: AsRef<[u8]>>(
        raw_password: T,
    ) -> Result<Password, PasswordError> {
        let hashed_password = argon2::hash_encoded(
            raw_password.as_ref(),
            PasswordSalt::salt().as_bytes(),
            &Config::default(),
        )?;
        Ok(Password(hashed_password))
    }

    #[inline]
    pub fn verify_password<T: AsRef<[u8]>>(
        hashed_password: &str,
        password: T,
    ) -> Result<bool, PasswordError> {
        Ok(argon2::verify_encoded(hashed_password, password.as_ref())?)
    }

    #[inline]
    pub fn verify<T: AsRef<[u8]>>(
        &self,
        password: T,
    ) -> Result<bool, PasswordError> {
        Self::verify_password(&self.0, password)
    }
}

impl Into<String> for Password {
    fn into(self) -> String {
        self.0
    }
}

impl From<String> for Password {
    #[inline]
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Deref for Password {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<String> for Password {
    fn as_ref(&self) -> &String {
        &self.0
    }
}
