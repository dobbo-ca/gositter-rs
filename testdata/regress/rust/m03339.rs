h_theme(&ColorfulTheme::default())
		.with_prompt(question)
		.default(placeholder.to_string())
		.interact_text()
		.map_err(|e| wrap(e, "Failed to read confirm input"))
}

/// Like [`prompt_options`], but for a list of dynamically formatted
/// (non-`Copy`) string labels. Returns the *index* of the selected item
/// into `options` rather than a copy of it, so callers can associate each
/// label with a non-`Copy` value (e.g. a full registry entry) by keeping
/// a parallel `Vec`.
pub fn prompt_index(text: impl Into<String>, options: &[String]) -> Result<usize, WrappedError> {
	Select::with_theme(&ColorfulTheme::default())
		.with_prompt(text)
		.items(options)
		.default(0)
		.interact()
		.map_err(|e| wrap(e, "Failed to read select input"))
}
