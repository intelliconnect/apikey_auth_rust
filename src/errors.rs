use actix_web::{HttpResponse, ResponseError};
use std::convert::From;

#[derive(Debug)]
pub enum Myerror {
    RedisError(String),
    RawErr(String),
}

impl ResponseError for Myerror {
    fn error_response(&self) -> HttpResponse {
        match self {
            Myerror::RedisError(s) | Myerror::RawErr(s) => {
                HttpResponse::InternalServerError().body(s.to_owned())
            }
        }
    }
}

impl From<redis::RedisError> for Myerror {
    fn from(err: redis::RedisError) -> Self {
        Self::RedisError(err.to_string())
    }
}

impl From<String> for Myerror {
    fn from(err: String) -> Self {
        Self::RawErr(err)
    }
}

impl std::fmt::Display for Myerror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
