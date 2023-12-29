use std::sync::OnceLock;

use minijinja::{Environment, Expression};
use regex::Regex;
use serde::Deserialize;
use toml::Value;

use crate::utils::{Result, ValidateVariableError};

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
#[serde(deny_unknown_fields)]
pub struct Variable {
    pub default: Value, // FIXME: use custom enum that includes other fields
    pub prompt: String,
    pub choices: Option<Vec<String>>,
    pub range: Option<(i64, i64)>, // FIXME: use dedicated struct
    pub pattern: Option<Pattern>,
    pub condition: Option<Condition>,
}

impl Variable {
    pub(crate) fn validate(self) -> Result<Self, ValidateVariableError> {
        match &self.default {
            Value::String(_) | Value::Array(_) | Value::Integer(_) | Value::Boolean(_) => {}
            _ => {
                return Err(ValidateVariableError::UnsupportedType {
                    type_str: self.default.type_str(),
                })
            }
        }
        if let Some(choices) = &self.choices {
            if self.pattern.is_some() {
                return Err(ValidateVariableError::IllegalField {
                    field: "pattern with choices",
                    type_str: self.default.type_str(),
                });
            }
            if choices.is_empty() {
                return Err(ValidateVariableError::DefaultOutsideChoices);
            }
            match &self.default {
                Value::String(default) => {
                    if !default.is_empty() && !choices.iter().any(|choice| choice == default) {
                        return Err(ValidateVariableError::DefaultOutsideChoices);
                    }
                }
                Value::Array(default) => {
                    if !default.is_empty()
                        && default.iter().any(|d| match d.as_str() {
                            None => true,
                            Some(d) => !choices.contains(&d.to_string()),
                        })
                    {
                        return Err(ValidateVariableError::DefaultOutsideChoices);
                    }
                }
                _ => {
                    return Err(ValidateVariableError::IllegalField {
                        field: "choices",
                        type_str: self.default.type_str(),
                    });
                }
            }
        }
        if let Some((min, max)) = &self.range {
            match self.default {
                Value::Integer(default) => {
                    if min >= max || default < *min || default > *max {
                        return Err(ValidateVariableError::UnreasonableRange);
                    }
                }
                _ => {
                    return Err(ValidateVariableError::IllegalField {
                        field: "range",
                        type_str: self.default.type_str(),
                    });
                }
            }
        }
        if self.pattern.is_some() {
            match self.default {
                Value::String(_) => {}
                _ => {
                    return Err(ValidateVariableError::IllegalField {
                        field: "pattern",
                        type_str: self.default.type_str(),
                    });
                }
            }
        }
        Ok(self)
    }
}
