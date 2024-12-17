use std::fmt::Display;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error while processing the geozero source.")]
    GeozeroError(#[from] geozero::error::GeozeroError),

    #[error("Serde error.")]
    SerdeError(#[from] serde_json::error::Error),

    #[error("An error happend.")]
    Message(String),

    #[error("Unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Message(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Message(msg.to_string())
    }
}
