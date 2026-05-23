//! Code Generation Test Suite Library
//!
//! This library provides math utility functions for testing codex-rs
//! code generation capabilities.

pub fn add(a: f64, b: f64) -> f64 {
    a + b
}

pub fn subtract(a: f64, b: f64) -> f64 {
    a - b
}

pub fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

pub fn divide(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        panic!("Division by zero");
    }
    a / b
}

pub fn power(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

pub fn sqrt(x: f64) -> f64 {
    if x < 0.0 {
        panic!("Square root of negative number");
    }
    x.sqrt()
}

pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
