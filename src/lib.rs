#![feature(option_result_contains)]
#![feature(int_abs_diff)]
#![feature(let_else)]
#![feature(bool_to_option)]

#[cfg(test)]
mod tests;

mod moves_calculator;

pub mod easing;
pub mod model;
pub mod ui;

pub mod systems {
    pub mod orbit_camera;
    pub mod chess;
}
