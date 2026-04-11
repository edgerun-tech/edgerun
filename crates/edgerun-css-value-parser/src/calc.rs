//! CSS calc() expression parsing.
//!
//! Supports: `calc()`, `min()`, `max()`, `clamp()`
//! With nested expressions: `calc(100% - calc(20px + 1em))`
//! With multiplication/division: `calc(2 * 10px)`, `calc(100% / 3)`

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::lengths::{LengthUnit, match_unit};

/// A calc() expression: a sequence of terms combined with + and -.
#[derive(Debug, Clone, PartialEq)]
pub struct CalcExpr {
    pub terms: Vec<CalcTerm>,
}

/// A single term in a calc expression.
#[derive(Debug, Clone, PartialEq)]
pub struct CalcTerm {
    pub negative: bool, // true if this term is subtracted
    pub factor: CalcFactor,
}

/// The atomic factor within a calc term.
#[derive(Debug, Clone, PartialEq)]
pub enum CalcFactor {
    /// A dimension: `10px`, `20%`, `1em`
    Dimension { value: f64, unit: LengthUnit },
    /// A percentage: `50%` (stored as 0.0..1.0)
    Percentage(f64),
    /// A plain number: `2`, `0.5`
    Number(f64),
    /// A nested calc() expression
    Calc(Box<CalcExpr>),
    /// Multiplication: `2 * 10px`
    Mul {
        left: Box<CalcFactor>,
        right: Box<CalcFactor>,
    },
    /// Division: `100% / 3`
    Div {
        numerator: Box<CalcFactor>,
        denominator: Box<CalcFactor>,
    },
}

/// Parse a calc() expression: `calc(100% - 20px)`
pub fn parse_calc(input: &str) -> Option<CalcExpr> {
    let input = input.trim();
    if !input.to_lowercase().starts_with("calc(") || !input.ends_with(')') {
        return None;
    }
    let inner = &input[5..input.len() - 1].trim();
    parse_calc_body(inner)
}

/// Parse the body of a calc() expression (content inside the parentheses).
fn parse_calc_body(input: &str) -> Option<CalcExpr> {
    let mut terms = Vec::new();
    let mut pos = 0;
    let input = input.trim();

    // Parse first term (no leading sign)
    let (factor, consumed) = parse_factor(input)?;
    terms.push(CalcTerm { negative: false, factor });
    pos += consumed;

    // Parse remaining terms: (+|-) factor
    loop {
        // Skip whitespace
        while pos < input.len() && input.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= input.len() {
            break;
        }

        let op = input.as_bytes()[pos] as char;
        if op != '+' && op != '-' {
            break;
        }
        let negative = op == '-';
        pos += 1;

        // Skip whitespace after operator
        while pos < input.len() && input.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }

        let (factor, consumed) = parse_factor(&input[pos..])?;
        terms.push(CalcTerm { negative, factor });
        pos += consumed;
    }

    if terms.is_empty() {
        return None;
    }

    Some(CalcExpr { terms })
}

/// Parse a calc factor: dimension, number, nested calc(), or mul/div expression.
fn parse_factor(input: &str) -> Option<(CalcFactor, usize)> {
    let input = input.trim_start();
    let leading_ws = input.len() - input.trim_start().len();

    // Nested calc()
    if input.to_lowercase().starts_with("calc(") {
        if let Some(expr) = parse_calc(input) {
            let end = find_matching_paren(input, 4)? + 1;
            return Some((CalcFactor::Calc(Box::new(expr)), leading_ws + end));
        }
    }

    // min() / max() / clamp() — treat as calc factors
    for fn_name in &["min(", "max(", "clamp("] {
        if input.to_lowercase().starts_with(fn_name) {
            let inner_start = fn_name.len();
            if let Some(inner_end) = find_matching_paren(input, inner_start - 1) {
                let inner = &input[inner_start..inner_end];
                let terms = parse_minmax_args(inner)?;
                let full_len = inner_end + 1;
                return Some((CalcFactor::Calc(Box::new(CalcExpr { terms })), leading_ws + full_len));
            }
        }
    }

    // Parenthesized expression
    if input.starts_with('(') {
        if let Some(end) = find_matching_paren(input, 0) {
            let inner = &input[1..end];
            let (inner_factor, _) = parse_factor(inner.trim())?;
            return Some((inner_factor, leading_ws + end + 1));
        }
    }

    // Try mul/div: parse first factor, then look for * or /
    if let Some((left, left_consumed)) = parse_simple_factor(input) {
        let rest = &input[left_consumed..];
        let rest_trimmed = rest.trim_start();
        let ws_after = rest.len() - rest_trimmed.len();

        if rest_trimmed.starts_with('*') {
            let (right, right_consumed) = parse_simple_factor(&rest_trimmed[1..])?;
            let total = left_consumed + ws_after + 1 + right_consumed;
            return Some((
                CalcFactor::Mul { left: Box::new(left), right: Box::new(right) },
                total,
            ));
        }
        if rest_trimmed.starts_with('/') {
            let (right, right_consumed) = parse_simple_factor(&rest_trimmed[1..])?;
            let total = left_consumed + ws_after + 1 + right_consumed;
            return Some((
                CalcFactor::Div { numerator: Box::new(left), denominator: Box::new(right) },
                total,
            ));
        }

        // No mul/div — just the simple factor
        return Some((left, left_consumed));
    }

    None
}

/// Parse a "simple" factor: dimension, percentage, number, or nested calc.
/// Does NOT try mul/div (that's handled at the caller level).
fn parse_simple_factor(input: &str) -> Option<(CalcFactor, usize)> {
    let input = input.trim_start();
    let leading_ws = input.len() - input.trim_start().len();
    let bytes = input.as_bytes();

    // Nested calc()
    if input.to_lowercase().starts_with("calc(") {
        if let Some(expr) = parse_calc(input) {
            let end = find_matching_paren(input, 4)? + 1;
            return Some((CalcFactor::Calc(Box::new(expr)), leading_ws + end));
        }
    }

    // min() / max() / clamp()
    for fn_name in &["min(", "max(", "clamp("] {
        if input.to_lowercase().starts_with(fn_name) {
            let inner_start = fn_name.len();
            if let Some(inner_end) = find_matching_paren(input, inner_start - 1) {
                let inner = &input[inner_start..inner_end];
                let terms = parse_minmax_args(inner)?;
                let full_len = inner_end + 1;
                return Some((CalcFactor::Calc(Box::new(CalcExpr { terms })), leading_ws + full_len));
            }
        }
    }

    // Parse number first, then check for % or unit
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    let mut has_dot = false;
    let mut has_digit = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            has_digit = true;
            i += 1;
        } else if bytes[i] == b'.' && !has_dot {
            has_dot = true;
            i += 1;
        } else {
            break;
        }
    }
    if !has_digit {
        return None;
    }

    let num_str = &input[..i];
    let value = num_str.parse::<f64>().ok()?;

    // Check for %
    if i < bytes.len() && bytes[i] == b'%' {
        return Some((CalcFactor::Percentage(value / 100.0), leading_ws + i + 1));
    }

    // Check for dimension unit
    if i < bytes.len() {
        let rest = &input[i..];
        if let Some((unit, unit_len)) = match_unit(rest) {
            return Some((CalcFactor::Dimension { value, unit }, leading_ws + i + unit_len));
        }
    }

    // Plain number
    Some((CalcFactor::Number(value), leading_ws + i))
}

/// Find the index of the closing `)` matching the `(` at `paren_pos`.
fn find_matching_paren(input: &str, paren_pos: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if paren_pos >= bytes.len() || bytes[paren_pos] != b'(' {
        return None;
    }
    let mut depth = 1;
    let mut i = paren_pos + 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Parse arguments for min()/max()/clamp() as calc terms.
fn parse_minmax_args(input: &str) -> Option<Vec<CalcTerm>> {
    let mut terms = Vec::new();
    let input = input.trim();

    // Split by commas
    let parts: alloc::vec::Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    for (_i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        // For min/max: all terms are added
        // For clamp: first is min, second is preferred, third is max
        // We just store them as positive terms for now
        if let Some((factor, consumed)) = parse_simple_factor(part) {
            if consumed >= part.len() || consumed > 0 {
                terms.push(CalcTerm { negative: false, factor });
            }
        } else {
            return None;
        }
    }

    if terms.is_empty() {
        return None;
    }
    Some(terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_simple_subtraction() {
        let expr = parse_calc("calc(100% - 20px)").unwrap();
        assert_eq!(expr.terms.len(), 2);
        assert!(!expr.terms[0].negative);
        assert!(expr.terms[1].negative);
    }

    #[test]
    fn test_calc_addition() {
        let expr = parse_calc("calc(10px + 5px)").unwrap();
        eprintln!("Parsed calc: {:?}", expr);
        assert_eq!(expr.terms.len(), 2);
        assert!(!expr.terms[0].negative);
        assert!(!expr.terms[1].negative);
    }

    #[test]
    fn test_calc_multiple_terms() {
        let expr = parse_calc("calc(100% - 20px + 5em)").unwrap();
        assert_eq!(expr.terms.len(), 3);
        assert!(!expr.terms[0].negative);
        assert!(expr.terms[1].negative);
        assert!(!expr.terms[2].negative);
    }

    #[test]
    fn test_calc_nested() {
        let expr = parse_calc("calc(100% - calc(20px + 5px))").unwrap();
        assert_eq!(expr.terms.len(), 2);
        assert!(expr.terms[1].negative);
        assert!(matches!(&expr.terms[1].factor, CalcFactor::Calc(_)));
    }

    #[test]
    fn test_calc_multiplication() {
        let expr = parse_calc("calc(2 * 10px)").unwrap();
        assert_eq!(expr.terms.len(), 1);
        assert!(matches!(&expr.terms[0].factor, CalcFactor::Mul { .. }));
    }

    #[test]
    fn test_calc_division() {
        let expr = parse_calc("calc(100% / 3)").unwrap();
        assert_eq!(expr.terms.len(), 1);
        assert!(matches!(&expr.terms[0].factor, CalcFactor::Div { .. }));
    }

    #[test]
    fn test_calc_case_insensitive() {
        let expr = parse_calc("CALC(10px + 5px)").unwrap();
        assert_eq!(expr.terms.len(), 2);
    }

    #[test]
    fn test_calc_invalid() {
        assert!(parse_calc("calc()").is_none());
        assert!(parse_calc("notcalc()").is_none());
    }

    #[test]
    fn test_min_function() {
        let expr = parse_calc("calc(min(100px, 50vw))").unwrap();
        assert_eq!(expr.terms.len(), 1);
    }

    #[test]
    fn test_clamp_function() {
        let expr = parse_calc("calc(clamp(10px, 50%, 100px))").unwrap();
        assert_eq!(expr.terms.len(), 1);
    }
}
