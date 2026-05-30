mod app;
mod cli;
mod config;
mod domain;
mod generator;
mod input;
mod io;
mod ui;

#[cfg(test)]
mod integration_tests;

pub use cli::run;
