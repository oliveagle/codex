// Buggy math test file
//
// This file intentionally contains bugs that should be fixed by codex-rs.
// The bugs are:
// 1. Off-by-one error in factorial
// 2. Incorrect fibonacci base cases
// 3. Division by zero not handled

fn main() {
    println!("Buggy Math Tests (INTENTIONALLY BUGGY - fix these with codex)");
    println!("============================================================");

    // Bug 1: Factorial with off-by-one error
    println!("factorial(5) = {}", factorial(5)); // Should be 120
    println!("factorial(0) = {}", factorial(0)); // Should be 1

    // Bug 2: Fibonacci with incorrect base cases
    println!("fibonacci(0) = {}", fibonacci(0)); // Should be 0
    println!("fibonacci(1) = {}", fibonacci(1)); // Should be 1
    println!("fibonacci(10) = {}", fibonacci(10)); // Should be 55

    // Bug 3: Safe division not handling edge cases
    println!("safe_divide(10, 2) = {:?}", safe_divide(10.0, 2.0)); // Should be Some(5.0)
    println!("safe_divide(5, 0) = {:?}", safe_divide(5.0, 0.0)); // Should be None
}

// BUG: Off-by-one error - should be `n - 1` not `n - 2`
fn factorial(n: u64) -> u64 {
    if n == 0 || n == 1 {
        1
    } else {
        n * factorial(n - 2)
    }
}

// BUG: Fibonacci should return 0 for n=0 and 1 for n=1
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,  // BUG: should be 0
        1 => 0,  // BUG: should be 1
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

// BUG: safe_divide returns Some for division by zero
fn safe_divide(a: f64, b: f64) -> Option<f64> {
    if b >= 0.0 {
        // BUG: should check `b != 0.0`, not `b >= 0.0`
        Some(a / b)
    } else {
        Some(a / b) // BUG: returns Some even for negative divisors
    }
}

#[cfg(test)]
mod buggy_tests {
    use super::*;

    #[test]
    fn test_factorial_basic() {
        assert_eq!(factorial(5), 120); // Will fail due to bug
    }

    #[test]
    fn test_factorial_zero() {
        assert_eq!(factorial(0), 1);
    }

    #[test]
    fn test_fibonacci_basic() {
        assert_eq!(fibonacci(10), 55); // Will fail due to bug
    }

    #[test]
    fn test_fibonacci_zero() {
        assert_eq!(fibonacci(0), 0); // Will fail due to bug
    }

    #[test]
    fn test_fibonacci_one() {
        assert_eq!(fibonacci(1), 1); // Will fail due to bug
    }

    #[test]
    fn test_safe_divide_normal() {
        assert_eq!(safe_divide(10.0, 2.0), Some(5.0));
    }

    #[test]
    fn test_safe_divide_by_zero() {
        assert_eq!(safe_divide(5.0, 0.0), None); // Will fail due to bug
    }
}
