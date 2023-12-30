use std::sync::OnceLock;

use minijinja::{Environment, Expression};
use regex::Regex;
use serde::Deserialize;

use crate::utils::{InvalidVariableError, Result};

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Pattern(Regex);

impl TryFrom<String> for Pattern {
    type Error = regex::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(Regex::new(&value)?))
    }
}

impl Pattern {
    pub fn is_match(&self, value: &str) -> bool {
        self.0.is_match(value)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Condition(Expression<'static, 'static>);

impl TryFrom<String> for Condition {
    type Error = minijinja::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();
        let environment = ENVIRONMENT.get_or_init(Environment::empty);
        Ok(Self(environment.compile_expression_owned(value)?))
    }
}

impl Condition {
    pub fn eval<S: serde::Serialize>(&self, ctx: S) -> Result<minijinja::Value, minijinja::Error> {
        self.0.eval(ctx)
    }
}

#[derive(Debug, Deserialize)]
// #[serde(deny_unknown_fields)]
pub struct Variable {
    #[serde(flatten)]
    pub value: VariableValue,
    pub prompt: String,
    pub condition: Option<Condition>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum VariableValue {
    String {
        default: String,
        pattern: Option<Pattern>,
        choices: Option<Vec<String>>,
    },
    Array {
        default: Vec<String>,
        choices: Vec<String>,
    },
    Integer {
        default: i64,
        range: Option<(i64, i64)>,
    },
    Boolean {
        default: bool,
    },
}

impl Variable {
    pub fn validate(self) -> Result<Self, InvalidVariableError> {
        match &self.value {
            VariableValue::String {
                default,
                pattern,
                choices,
            } => {
                if let Some(choices) = choices {
                    if pattern.is_some() {
                        return Err(InvalidVariableError::PatternWithChoices);
                    }
                    if choices.is_empty() {
                        return Err(InvalidVariableError::DefaultOutsideChoices);
                    }
                    if !default.is_empty() && !choices.iter().any(|choice| choice == default) {
                        return Err(InvalidVariableError::DefaultOutsideChoices);
                    }
                } else if let Some(pattern) = pattern {
                    if !default.is_empty() && !pattern.is_match(default) {
                        return Err(InvalidVariableError::DefaultMismatchPattern);
                    }
                }
            }
            VariableValue::Array { default, choices } => {
                if !default.is_empty() && default.iter().any(|d| !choices.contains(d)) {
                    return Err(InvalidVariableError::DefaultOutsideChoices);
                }
            }
            VariableValue::Integer {
                default,
                range: Some((min, max)),
            } => {
                if min >= max || default < min || default > max {
                    return Err(InvalidVariableError::UnreasonableRange);
                }
            }
            _ => {}
        }
        Ok(self)
    }
}
