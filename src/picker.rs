use std::fmt::Display;

use dialoguer::{FuzzySelect, theme::ColorfulTheme};

pub(crate) fn pick_interactively<T: Display>(items: Vec<T>, prompt: &str) -> T {
    // With a single option there is nothing to choose: pick it directly so
    // non-interactive runs (and tests) don't block on a tty prompt.
    if items.len() == 1 {
        return items.into_iter().next().expect("Internal Error");
    }

    let index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()
        .unwrap();

    items.into_iter().nth(index).expect("Internal Error")
}
