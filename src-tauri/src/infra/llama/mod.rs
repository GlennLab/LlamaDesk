//! llama.cpp router adapter: DTOs, Prometheus parsing, and the HTTP client.

pub mod client;
pub mod dto;
pub mod prometheus;

pub use client::{
    DEFAULT_CONNECT_TIMEOUT, DEFAULT_TIMEOUT, LlamaRouterClient, LlamaRouterClientBuilder,
};
