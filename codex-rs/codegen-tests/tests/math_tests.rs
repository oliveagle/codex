// Unit tests for math operations
//
// These tests verify that the generated code functions correctly.
// Run with: cargo test

// Import functions from the main module
extern crate codex_codegen_tests;
use codex_codegen_tests::{add, subtract, multiply, divide, power, sqrt, clamp, lerp};

#[cfg(test)]
mod math_tests {
    use super::*;

    // Basic addition tests
    #[test]
    fn test_add_positive_numbers() {
        assert_eq!(add(2.0, 3.0), 5.0);
        assert_eq!(add(0.0, 0.0), 0.0);
        assert_eq!(add(-2.0, 3.0), 1.0);
    }

    #[test]
    fn test_add_negative_numbers() {
        assert_eq!(add(-2.0, -3.0), -5.0);
        assert_eq!(add(-5.0, 5.0), 0.0);
    }

    // Subtraction tests
    #[test]
    fn test_subtract_positive_numbers() {
        assert_eq!(subtract(5.0, 3.0), 2.0);
        assert_eq!(subtract(3.0, 5.0), -2.0);
    }

    #[test]
    fn test_subtract_zero() {
        assert_eq!(subtract(5.0, 0.0), 5.0);
        assert_eq!(subtract(0.0, 5.0), -5.0);
    }

    // Multiplication tests
    #[test]
    fn test_multiply_positive_numbers() {
        assert_eq!(multiply(3.0, 4.0), 12.0);
        assert_eq!(multiply(0.0, 5.0), 0.0);
    }

    #[test]
    fn test_multiply_negative_numbers() {
        assert_eq!(multiply(-2.0, 3.0), -6.0);
        assert_eq!(multiply(-2.0, -3.0), 6.0);
    }

    // Division tests
    #[test]
    fn test_divide_positive_numbers() {
        assert_eq!(divide(10.0, 2.0), 5.0);
        assert_eq!(divide(7.0, 2.0), 3.5);
    }

    #[test]
    fn test_divide_negative_numbers() {
        assert_eq!(divide(-10.0, 2.0), -5.0);
        assert_eq!(divide(10.0, -2.0), -5.0);
    }

    #[test]
    #[should_panic(expected = "Division by zero")]
    fn test_divide_by_zero_panics() {
        divide(5.0, 0.0);
    }

    // Power function tests
    #[test]
    fn test_power_positive() {
        assert!((power(2.0, 3.0) - 8.0).abs() < f64::EPSILON);
        assert!((power(5.0, 2.0) - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_power_zero() {
        assert!((power(5.0, 0.0) - 1.0).abs() < f64::EPSILON);
        assert!((power(0.0, 5.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_power_negative_exponent() {
        assert!((power(2.0, -1.0) - 0.5).abs() < f64::EPSILON);
    }

    // Square root tests
    #[test]
    fn test_sqrt_positive_numbers() {
        assert!((sqrt(16.0) - 4.0).abs() < f64::EPSILON);
        assert!((sqrt(2.0) - 1.41421356237).abs() < 1e-9);
    }

    #[test]
    fn test_sqrt_zero() {
        assert!((sqrt(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    #[should_panic(expected = "Square root of negative number")]
    fn test_sqrt_negative_panics() {
        sqrt(-1.0);
    }

    // Clamp function tests
    #[test]
    fn test_clamp_within_range() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(0.5, 0.0, 1.0), 0.5);
    }

    #[test]
    fn test_clamp_below_minimum() {
        assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(0.0, 5.0, 10.0), 5.0);
    }

    #[test]
    fn test_clamp_above_maximum() {
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
        assert_eq!(clamp(10.0, 0.0, 5.0), 5.0);
    }

    // Linear interpolation tests
    #[test]
    fn test_lerp_at_extremes() {
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < f64::EPSILON);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lerp_midpoint() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lerp_beyond_range() {
        assert!((lerp(0.0, 10.0, -0.5) - (-5.0)).abs() < f64::EPSILON);
        assert!((lerp(0.0, 10.0, 1.5) - 15.0).abs() < f64::EPSILON);
    }
}
