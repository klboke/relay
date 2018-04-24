//! Implements basics for the protocol.
#![warn(missing_docs)]
extern crate actix;
extern crate actix_web;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate futures;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate parking_lot;
extern crate regex;
extern crate sentry_types;
extern crate serde;
extern crate serde_json;
extern crate smith_aorta;
extern crate smith_common;
extern crate smith_config;
extern crate smith_trove;
extern crate tokio_core;

mod service;
mod errors;
mod utils;
mod extractors;

pub use service::*;
pub use errors::*;
