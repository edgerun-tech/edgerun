//! Layer 15: Accessibility Conformance Analyzer
//!
//! Static analysis of CSS against WCAG 2.2 rules — no browser needed,
//! just the style data. We have every CSS property as structured data
//! with computed values. Other browsers resolve styles at render time
//! — too late for static analysis.

use std::collections::BTreeMap;

/// WCAG 2.2 conformance level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub enum ConformanceLevel { A, AA, AAA }

/// Result of an accessibility check.
#[derive(Clone, Debug)]
pub struct A11yIssue {
    pub severity: Severity,
    pub rule: &'static str,
    pub wcag_criterion: &'static str,
    pub level: ConformanceLevel,
    pub message: String,
    pub element: String,
    pub suggestion: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity { Error, Warning, Info }

/// Computed style for an element (simplified for static analysis).
#[derive(Clone, Debug)]
pub struct ElementStyle {
    pub tag: String,
    pub color_r: f32, pub color_g: f32, pub color_b: f32,
    pub bg_r: f32, pub bg_g: f32, pub bg_b: f32,
    pub font_size_px: f32,
    pub font_weight: f32,
    pub line_height_px: f32,
    pub letter_spacing_px: f32,
    pub animation_duration_ms: f32,
    pub transition_duration_ms: f32,
    pub outline_style: String,
    pub outline_color_r: f32, pub outline_color_g: f32, pub outline_color_b: f32,
    pub is_focusable: bool,
    pub has_hover_style: bool,
    pub text: String,
}

/// Full accessibility analysis result.
#[derive(Debug)]
pub struct A11yReport {
    pub issues: Vec<A11yIssue>,
    pub passed_checks: usize,
    pub total_checks: usize,
}

impl A11yReport {
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Error).count()
    }
    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).count()
    }
    pub fn conformance_level(&self) -> ConformanceLevel {
        if self.error_count() == 0 { ConformanceLevel::AA }
        else { ConformanceLevel::A }
    }

    pub fn print_summary(&self) {
        println!("┌─ Accessibility Report ────────────────────────────");
        println!("│ Passed: {}/{} checks", self.passed_checks, self.total_checks);
        println!("│ Errors:   {}", self.error_count());
        println!("│ Warnings: {}", self.warning_count());
        println!("│ Level: {:?}", self.conformance_level());
        println!("│");
        for issue in &self.issues {
            let icon = match issue.severity {
                Severity::Error => "❌",
                Severity::Warning => "⚠️ ",
                Severity::Info => "ℹ️ ",
            };
            println!("│ {} [{}] {}: {}", icon, issue.wcag_criterion, issue.rule, issue.message);
            println!("│   Element: <{}>", issue.element);
            if !issue.suggestion.is_empty() {
                println!("│   Fix: {}", issue.suggestion);
            }
        }
        println!("└────────────────────────────────────────────────────");
    }
}

/// Run full accessibility analysis on a set of styled elements.
pub fn analyze(styles: &[ElementStyle]) -> A11yReport {
    let mut issues = Vec::new();
    let mut passed = 0;
    let mut total = 0;

    for style in styles {
        // 1.1.1: Contrast ratio (WCAG 2.2, 1.4.3 — AA)
        total += 1;
        let contrast = relative_luminance_ratio(
            (style.color_r, style.color_g, style.color_b),
            (style.bg_r, style.bg_g, style.bg_b),
        );
        if contrast >= 4.5 {
            passed += 1;
        } else {
            issues.push(A11yIssue {
                severity: Severity::Error,
                rule: "contrast-ratio",
                wcag_criterion: "1.4.3",
                level: ConformanceLevel::AA,
                message: format!("Contrast ratio {:.1}:1 on <{}> (needs 4.5:1 for AA)",
                    contrast, style.tag),
                element: style.tag.clone(),
                suggestion: format!("Increase contrast: use darker text or lighter background"),
            });
        }

        // 1.4.6: Enhanced contrast (AAA)
        total += 1;
        if contrast >= 7.0 {
            passed += 1;
        } else {
            issues.push(A11yIssue {
                severity: Severity::Warning,
                rule: "contrast-enhanced",
                wcag_criterion: "1.4.6",
                level: ConformanceLevel::AAA,
                message: format!("Contrast ratio {:.1}:1 (needs 7:1 for AAA)", contrast),
                element: style.tag.clone(),
                suggestion: String::new(),
            });
        }

        // 1.4.4: Resize text (minimum 12px)
        total += 1;
        if style.font_size_px >= 12.0 {
            passed += 1;
        } else {
            issues.push(A11yIssue {
                severity: Severity::Error,
                rule: "min-font-size",
                wcag_criterion: "1.4.4",
                level: ConformanceLevel::A,
                message: format!("Font-size {:.0}px on <{}> (below 12px minimum)",
                    style.font_size_px, style.tag),
                element: style.tag.clone(),
                suggestion: "Set font-size to at least 12px".into(),
            });
        }

        // 1.4.12: Text spacing (line-height >= 1.5 * font-size)
        total += 1;
        let min_line_height = style.font_size_px * 1.5;
        if style.line_height_px >= min_line_height {
            passed += 1;
        } else {
            issues.push(A11yIssue {
                severity: Severity::Warning,
                rule: "text-spacing",
                wcag_criterion: "1.4.12",
                level: ConformanceLevel::AA,
                message: format!("Line-height {:.0}px on <{}> (should be ≥ {:.0}px)",
                    style.line_height_px, style.tag, min_line_height),
                element: style.tag.clone(),
                suggestion: format!("Set line-height to at least {:.0}px", min_line_height),
            });
        }

        // 2.3.1: No flashing (animation duration)
        total += 1;
        if style.animation_duration_ms == 0.0 || style.animation_duration_ms >= 200.0 {
            passed += 1;
        } else {
            issues.push(A11yIssue {
                severity: Severity::Error,
                rule: "animation-duration",
                wcag_criterion: "2.3.1",
                level: ConformanceLevel::A,
                message: format!("Animation duration {}ms on <{}> (below 200ms threshold)",
                    style.animation_duration_ms, style.tag),
                element: style.tag.clone(),
                suggestion: "Set animation duration to at least 200ms or remove animation".into(),
            });
        }

        // 2.4.7: Focus visible (focusable elements need outline)
        if style.is_focusable {
            total += 1;
            if style.outline_style != "none" && style.outline_style.is_empty() == false {
                passed += 1;
            } else {
                issues.push(A11yIssue {
                    severity: Severity::Error,
                    rule: "focus-visible",
                    wcag_criterion: "2.4.7",
                    level: ConformanceLevel::AA,
                    message: format!("No focus indicator on <{}>", style.tag),
                    element: style.tag.clone(),
                    suggestion: "Add outline or outline-style to focusable elements".into(),
                });
            }
        }

        // 1.4.1: Use of color (text should not rely solely on color)
        total += 1;
        // Heuristic: if text is short and contains only color differences, warn
        if style.text.len() > 3 {
            passed += 1; // Assume content provides non-color distinction
        } else {
            issues.push(A11yIssue {
                severity: Severity::Info,
                rule: "use-of-color",
                wcag_criterion: "1.4.1",
                level: ConformanceLevel::A,
                message: format!("Short text on <{}> — verify meaning isn't conveyed by color alone",
                    style.tag),
                element: style.tag.clone(),
                suggestion: String::new(),
            });
            passed += 1; // Not a real failure, just info
        }
    }

    A11yReport {
        issues,
        passed_checks: passed,
        total_checks: total,
    }
}

/// Calculate relative luminance ratio between two colors.
/// Based on WCAG 2.2 formula: (L1 + 0.05) / (L2 + 0.05)
fn relative_luminance_ratio(c1: (f32, f32, f32), c2: (f32, f32, f32)) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Relative luminance per WCAG 2.2.
fn relative_luminance((r, g, b): (f32, f32, f32)) -> f32 {
    fn linearize(c: f32) -> f32 {
        let s = c / 255.0;
        if s <= 0.03928 { s / 12.92 }
        else { ((s + 0.055) / 1.055).powf(2.4) }
    }
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_good_style() -> ElementStyle {
        ElementStyle {
            tag: "p".into(),
            color_r: 0.0, color_g: 0.0, color_b: 0.0,
            bg_r: 255.0, bg_g: 255.0, bg_b: 255.0,
            font_size_px: 16.0, font_weight: 400.0,
            line_height_px: 24.0, letter_spacing_px: 0.0,
            animation_duration_ms: 0.0, transition_duration_ms: 0.0,
            outline_style: "solid".into(),
            outline_color_r: 0.0, outline_color_g: 0.0, outline_color_b: 255.0,
            is_focusable: true, has_hover_style: false,
            text: "Hello world".into(),
        }
    }

    #[test]
    fn test_good_style_passes() {
        let style = make_good_style();
        let report = analyze(&[style]);
        assert_eq!(report.error_count(), 0, "Good style should have 0 errors");
        assert!(report.conformance_level() >= ConformanceLevel::AA);
    }

    #[test]
    fn test_low_contrast_fails() {
        let mut style = make_good_style();
        // Light gray on white — low contrast
        style.color_r = 153.0; style.color_g = 153.0; style.color_b = 153.0;
        style.bg_r = 255.0; style.bg_g = 255.0; style.bg_b = 255.0;

        let report = analyze(&[style]);
        assert!(report.error_count() > 0, "Low contrast should fail");
        let contrast_issue = report.issues.iter().find(|i| i.rule == "contrast-ratio");
        assert!(contrast_issue.is_some(), "Should have contrast-ratio issue");
    }

    #[test]
    fn test_small_font_fails() {
        let mut style = make_good_style();
        style.font_size_px = 10.0;

        let report = analyze(&[style]);
        let font_issue = report.issues.iter().find(|i| i.rule == "min-font-size");
        assert!(font_issue.is_some(), "Small font should fail min-font-size");
    }

    #[test]
    fn test_fast_animation_fails() {
        let mut style = make_good_style();
        style.animation_duration_ms = 50.0;

        let report = analyze(&[style]);
        let anim_issue = report.issues.iter().find(|i| i.rule == "animation-duration");
        assert!(anim_issue.is_some(), "Fast animation should fail");
    }

    #[test]
    fn test_no_focus_outline_fails() {
        let mut style = make_good_style();
        style.is_focusable = true;
        style.outline_style = "none".into();

        let report = analyze(&[style]);
        let focus_issue = report.issues.iter().find(|i| i.rule == "focus-visible");
        assert!(focus_issue.is_some(), "No focus outline should fail");
    }

    #[test]
    fn test_luminance_calculation() {
        // White vs black = 21:1
        let ratio = relative_luminance_ratio((255.0, 255.0, 255.0), (0.0, 0.0, 0.0));
        assert!((ratio - 21.0).abs() < 0.1, "White vs black should be ~21:1, got {:.1}", ratio);

        // Same color = 1:1
        let ratio = relative_luminance_ratio((128.0, 128.0, 128.0), (128.0, 128.0, 128.0));
        assert!((ratio - 1.0).abs() < 0.01, "Same color should be 1:1, got {:.2}", ratio);
    }
}
