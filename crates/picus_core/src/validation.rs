//! Input validation / form validation for ECS-driven UI.
//!
//! Provides components, resources, and systems for declarative validation
//! of user-input fields such as text inputs, sliders, and custom widgets.

use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// Helper: simple pattern matching without regex crate
// ---------------------------------------------------------------------------

/// Match a simple pattern against a value.
///
/// Supports basic patterns like `^[a-zA-Z]+$`, `^[0-9]+$`, `^[a-zA-Z0-9]+$`,
/// exact match (`^literal$`), and prefix/suffix/infix search.
fn matches_pattern(value: &str, pattern: &str) -> bool {
    match pattern {
        "^[a-zA-Z]+$" => !value.is_empty() && value.chars().all(|c| c.is_ascii_alphabetic()),
        "^[0-9]+$" => !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()),
        "^[a-zA-Z0-9]+$" => !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric()),
        "^[a-zA-Z0-9_]+$" => {
            !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        "^[a-zA-Z0-9_-]+$" => {
            !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        }
        "^[a-z]+$" => !value.is_empty() && value.chars().all(|c| c.is_ascii_lowercase()),
        "^[A-Z]+$" => !value.is_empty() && value.chars().all(|c| c.is_ascii_uppercase()),
        "^[a-zA-Z0-9\\s]+$" => {
            !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        }
        "^[\\w.@+-]+$" => {
            !value.is_empty()
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '@' | '+' | '-' | '_'))
        }
        _ => {
            // Exact match for "^literal$" patterns
            if let (Some(inner), true) = (pattern.strip_prefix('^'), true)
                && let Some(literal) = inner.strip_suffix('$')
            {
                return value == literal;
            }
            // Simple contains for other patterns
            let trimmed = pattern.trim_matches('^').trim_matches('$');
            value.contains(trimmed)
        }
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Categorised error code used to identify which rule was violated.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorCode {
    /// A required field was left empty.
    Required,
    /// The value is shorter than the allowed minimum.
    MinLength(usize),
    /// The value exceeds the allowed maximum.
    MaxLength(usize),
    /// The value does not match the required pattern.
    Pattern(String),
    /// The numeric value falls outside the allowed range.
    Range { min: f64, max: f64 },
    /// A user-defined error identified by a custom code.
    Custom(u32),
}

/// A single validation error or warning attached to a field.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Human-readable error message.
    pub message: String,
    /// The rule code that triggered this error.
    pub code: ValidationErrorCode,
    /// Optional i18n key for looking up a localised error message.
    pub i18n_key: Option<String>,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Runtime validation state attached to an input entity.
#[derive(Component, Debug, Clone)]
pub struct ValidationState {
    /// Current validation errors.
    pub errors: Vec<ValidationError>,
    /// Current validation warnings (non-blocking).
    pub warnings: Vec<ValidationError>,
    /// Whether the field is currently valid (no errors).
    pub is_valid: bool,
    /// Whether the user has interacted with this field at least once.
    pub touched: bool,
    /// Whether the value has changed but has not yet been re-validated.
    pub dirty: bool,
}

impl Default for ValidationState {
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            is_valid: true,
            touched: false,
            dirty: false,
        }
    }
}

/// Declarative validation rules for an input entity.
#[derive(Component, Debug, Clone, Default)]
pub struct ValidationRules {
    /// When `true`, a non-empty value is required.
    pub required: bool,
    /// Minimum string / collection length (inclusive).
    pub min_length: Option<usize>,
    /// Maximum string / collection length (inclusive).
    pub max_length: Option<usize>,
    /// Regex-like pattern the value must match (e.g. `"^[a-zA-Z]+$"`).
    pub pattern: Option<String>,
    /// Minimum numeric value (inclusive).
    pub min_value: Option<f64>,
    /// Maximum numeric value (inclusive).
    pub max_value: Option<f64>,
    /// Optional entity that implements a custom validation routine.
    pub custom_validator: Option<Entity>,
}

/// A string value that can be validated against [`ValidationRules`].
///
/// Input components (e.g. `UiTextInput`) should insert this so that the
/// `run_validation` system can check rules against the actual value.
#[derive(Component, Debug, Clone, Default)]
pub struct ValidatedString {
    /// The current field value to validate.
    pub value: String,
}

/// Controls how validation feedback is displayed for an entity.
#[derive(Component, Debug, Clone)]
pub struct ValidationDisplay {
    /// Whether to show error messages.
    pub show_errors: bool,
    /// Whether to show warning messages.
    pub show_warnings: bool,
    /// Optional style-class name applied when the entity has errors.
    pub error_style_class: Option<String>,
}

impl Default for ValidationDisplay {
    fn default() -> Self {
        Self {
            show_errors: true,
            show_warnings: true,
            error_style_class: Some("validation-error".to_string()),
        }
    }
}

/// Marker component that signals an entity needs re-validation.
///
/// Systems that modify a field's value should add this component to the
/// entity so that [`run_validation`] picks it up on the next frame.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NeedsValidation;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Global registry for validator metadata.
#[derive(Resource, Debug, Default)]
pub struct ValidationRegistry;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Runs validation for every entity whose [`NeedsValidation`] marker has
/// changed (e.g. was just added).
///
/// Reads [`ValidationRules`] and [`ValidatedString`] attached to the entity
/// and updates [`ValidationState`] accordingly.
#[allow(clippy::type_complexity, clippy::collapsible_if)]
pub fn run_validation(
    mut query: Query<
        (
            Entity,
            &ValidationRules,
            Option<&ValidatedString>,
            Option<&mut ValidationState>,
        ),
        Changed<NeedsValidation>,
    >,
    mut commands: Commands,
) {
    for (entity, rules, validated, state) in query.iter_mut() {
        let mut new_state = match state {
            Some(s) => s.clone(),
            None => ValidationState::default(),
        };

        new_state.errors.clear();
        new_state.warnings.clear();

        let field_value: Option<&str> = validated.map(|v| v.value.as_str());

        // --- Required check ---
        if rules.required {
            let is_empty = field_value.map(|v| v.trim().is_empty()).unwrap_or(true);
            if is_empty {
                new_state.errors.push(ValidationError {
                    message: "This field is required".to_string(),
                    code: ValidationErrorCode::Required,
                    i18n_key: Some("validation-required".to_string()),
                });
            }
        }

        // --- Min/Max length checks ---
        if let Some(value) = field_value {
            if let Some(min) = rules.min_length {
                if value.len() < min {
                    new_state.errors.push(ValidationError {
                        message: format!("Minimum {min} characters required"),
                        code: ValidationErrorCode::MinLength(min),
                        i18n_key: Some("validation-min-length".to_string()),
                    });
                }
            }
            if let Some(max) = rules.max_length {
                if value.len() > max {
                    new_state.errors.push(ValidationError {
                        message: format!("Maximum {max} characters allowed"),
                        code: ValidationErrorCode::MaxLength(max),
                        i18n_key: Some("validation-max-length".to_string()),
                    });
                }
            }

            // --- Pattern check ---
            if let Some(ref pattern_str) = rules.pattern {
                if !value.is_empty() && !matches_pattern(value, pattern_str) {
                    new_state.errors.push(ValidationError {
                        message: format!("Value does not match required pattern: {pattern_str}"),
                        code: ValidationErrorCode::Pattern(pattern_str.clone()),
                        i18n_key: Some("validation-pattern".to_string()),
                    });
                }
            }
        }

        new_state.is_valid = new_state.errors.is_empty();
        new_state.dirty = false;

        commands.entity(entity).insert(new_state);
        commands.entity(entity).remove::<NeedsValidation>();
    }
}

/// Clears the [`ValidationState`] on every entity that carries one.
pub fn clear_validation(mut query: Query<&mut ValidationState>) {
    for mut state in query.iter_mut() {
        state.errors.clear();
        state.warnings.clear();
        state.is_valid = true;
        state.touched = false;
        state.dirty = false;
    }
}

/// Marks every entity with [`ValidationState`] as dirty and touched.
pub fn mark_validation_dirty(mut query: Query<&mut ValidationState>) {
    for mut state in query.iter_mut() {
        state.dirty = true;
        state.touched = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_pattern_alphabetic() {
        assert!(matches_pattern("hello", "^[a-zA-Z]+$"));
        assert!(!matches_pattern("hello1", "^[a-zA-Z]+$"));
        assert!(!matches_pattern("", "^[a-zA-Z]+$"));
    }

    #[test]
    fn matches_pattern_numeric() {
        assert!(matches_pattern("12345", "^[0-9]+$"));
        assert!(!matches_pattern("12a45", "^[0-9]+$"));
    }

    #[test]
    fn matches_pattern_email() {
        assert!(matches_pattern("user@example.com", "^[\\w.@+-]+$"));
        assert!(!matches_pattern("user name", "^[\\w.@+-]+$"));
    }
}
