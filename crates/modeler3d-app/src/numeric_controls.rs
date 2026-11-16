//! Toolkit-neutral numeric-field selection behavior.
//!
//! Egui's numeric widgets do not expose the Qt line-edit subclass used by the
//! Python desktop. This helper preserves the contract at the text boundary:
//! select the clean numeric value while leaving a display suffix untouched.

use crate::preferences::DisplayUnit;

/// Returns the byte range of the clean numeric value inside a rendered field.
#[must_use]
pub fn select_numeric_value(text: &str, clean_value: &str) -> Option<(usize, usize)> {
    if clean_value.is_empty() {
        return None;
    }
    let start = text.find(clean_value)?;
    Some((start, start + clean_value.len()))
}

/// Parses a finite distance with an optional `mm` or `in` suffix.
///
/// The returned value is always millimetres so the model contract stays
/// authoritative even when the shell is displaying inches. A missing suffix
/// uses the active display unit; an incompatible or non-finite value is
/// rejected before it can reach a transform command.
pub fn parse_distance_mm(text: &str, display_unit: DisplayUnit) -> Result<f32, &'static str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("enter a value");
    }
    let lower = trimmed.to_ascii_lowercase();
    let (number, unit) = if let Some(value) = lower.strip_suffix("mm") {
        (value.trim(), DisplayUnit::Millimetre)
    } else if let Some(value) = lower.strip_suffix("in") {
        (value.trim(), DisplayUnit::Inch)
    } else {
        (lower.as_str(), display_unit)
    };
    let value = number.parse::<f32>().map_err(|_| "enter a finite number")?;
    if !value.is_finite() {
        return Err("enter a finite number");
    }
    let millimetres = unit.to_mm(value);
    if !millimetres.is_finite() {
        return Err("enter a finite number");
    }
    Ok(millimetres)
}

/// Formats a model distance using the selected display unit and a stable
/// suffix for keyboard editing.
#[must_use]
pub fn format_distance(value_mm: f32, display_unit: DisplayUnit) -> String {
    let value = display_unit.from_mm(value_mm);
    let value = if value.abs() < 0.000_5 { 0.0 } else { value };
    format!("{value:.3} {}", display_unit.label())
}

/// Formats an angle in degrees for a transform box. Values that would round
/// to zero show as "0.000", never "-0.000".
#[must_use]
pub fn format_angle_degrees(degrees: f32) -> String {
    let degrees = if degrees.abs() < 0.000_5 {
        0.0
    } else {
        degrees
    };
    format!("{degrees:.3} deg")
}

/// Parses a finite rotation delta in degrees.
pub fn parse_angle_degrees(text: &str) -> Result<f32, &'static str> {
    let trimmed = text.trim().trim_end_matches('°').trim();
    let trimmed = trimmed
        .strip_suffix("deg")
        .or_else(|| trimmed.strip_suffix("degrees"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let value = trimmed.parse::<f32>().map_err(|_| "enter a finite angle")?;
    value
        .is_finite()
        .then_some(value)
        .ok_or("enter a finite angle")
}

/// Parses a positive finite scale factor, accepting an optional `x` suffix.
pub fn parse_scale_factor(text: &str) -> Result<f32, &'static str> {
    let trimmed = text.trim().trim_end_matches('x').trim();
    let value = trimmed
        .parse::<f32>()
        .map_err(|_| "enter a finite scale factor")?;
    if !value.is_finite() || value <= 0.0 {
        return Err("scale must be greater than zero");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_excludes_numeric_suffix() {
        assert_eq!(select_numeric_value("12.50 mm", "12.50"), Some((0, 5)));
        assert_eq!(select_numeric_value("42", "42"), Some((0, 2)));
        assert_eq!(select_numeric_value("value", ""), None);
        assert_eq!(select_numeric_value("value", "missing"), None);
    }

    #[test]
    fn distance_parser_accepts_suffixes_and_rejects_bad_values() {
        assert_eq!(parse_distance_mm("12.5 mm", DisplayUnit::Inch), Ok(12.5));
        assert_eq!(parse_distance_mm("2 in", DisplayUnit::Millimetre), Ok(50.8));
        assert_eq!(parse_distance_mm("2", DisplayUnit::Inch), Ok(50.8));
        assert!(parse_distance_mm("NaN", DisplayUnit::Millimetre).is_err());
        assert!(parse_distance_mm("", DisplayUnit::Millimetre).is_err());
    }

    #[test]
    fn angles_near_zero_show_without_a_minus_sign() {
        assert_eq!(format_angle_degrees(-0.0), "0.000 deg");
        assert_eq!(format_angle_degrees(-0.000_2), "0.000 deg");
        assert_eq!(format_angle_degrees(-90.0), "-90.000 deg");
        assert_eq!(format_angle_degrees(12.345_6), "12.346 deg");
    }

    #[test]
    fn angle_and_scale_parsers_keep_units_explicit() {
        assert_eq!(parse_angle_degrees("-90 deg"), Ok(-90.0));
        assert_eq!(parse_angle_degrees("45°"), Ok(45.0));
        assert_eq!(parse_scale_factor("1.25x"), Ok(1.25));
        assert!(parse_scale_factor("0").is_err());
        assert!(parse_angle_degrees("NaN").is_err());
    }
}
