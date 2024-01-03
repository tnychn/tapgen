use std::str::FromStr;
use std::sync::OnceLock;

use dialoguer::theme::SimpleTheme;
use dialoguer::{Confirm, Input, InputValidator, MultiSelect, Select};

static THEME: OnceLock<SimpleTheme> = OnceLock::new();

pub(crate) fn select<P: Into<String>, T: ToString + Clone>(
    prompt: P,
    items: &[T],
    default: Option<T>,
) -> T {
    let theme = THEME.get_or_init(|| SimpleTheme);
    let mut p = Select::with_theme(theme).with_prompt(prompt).items(items);
    if let Some(default) = default {
        p = p.default(
            items
                .iter()
                .position(|item| item.to_string() == default.to_string())
                .unwrap(),
        )
    }
    items[p.interact().unwrap()].clone()
}

pub(crate) fn multi_select<P: Into<String>, T: ToString + Clone>(
    prompt: P,
    items: &[T],
    defaults: Option<&[T]>,
) -> Vec<T> {
    let theme = THEME.get_or_init(|| SimpleTheme);
    let mut p = MultiSelect::with_theme(theme)
        .with_prompt(prompt)
        .items(items);
    if let Some(defaults) = defaults {
        if !defaults.is_empty() {
            let defaults = defaults
                .iter()
                .map(|default| default.to_string())
                .collect::<Vec<String>>();
            p = p.defaults(
                &items
                    .iter()
                    .map(|choice| defaults.contains(&choice.to_string()))
                    .collect::<Vec<bool>>(),
            )
        }
    }
    p.interact()
        .unwrap()
        .iter()
        .map(|&i| items[i].clone())
        .collect()
}

pub(crate) fn confirm(prompt: impl Into<String>, default: Option<bool>) -> bool {
    let theme = THEME.get_or_init(|| SimpleTheme);
    let mut p = Confirm::with_theme(theme).with_prompt(prompt);
    if let Some(default) = default {
        p = p.default(default);
    }
    p.interact().unwrap()
}

pub(crate) fn input<'a, T: 'a, V>(
    prompt: impl Into<String>,
    default: Option<T>,
    validator: Option<V>,
) -> T
where
    T: Clone + ToString + FromStr,
    <T as FromStr>::Err: ToString,
    V: InputValidator<T> + 'a,
    V::Err: ToString,
{
    let theme = THEME.get_or_init(|| SimpleTheme);
    let mut p = Input::with_theme(theme).with_prompt(prompt);
    if let Some(default) = default {
        p = p.default(default)
    }
    if let Some(validator) = validator {
        p = p.validate_with(validator);
    }
    p.interact_text().unwrap()
}
