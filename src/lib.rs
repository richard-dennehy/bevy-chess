#![feature(option_result_contains)]
#![feature(int_abs_diff)]
#![feature(let_else)]
#![feature(format_args_capture)]
#![feature(bool_to_option)]

#[cfg(test)]
mod tests;

mod moves_calculator;

pub mod board;
pub mod pieces;
pub mod ui;
