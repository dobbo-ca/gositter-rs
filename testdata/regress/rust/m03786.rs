le) are resolved against the current
/// working directory, matching the TypeScript resolver's behavior of
/// resolving non-absolute paths against `process.cwd()`.
pub fn resolve_user_data_path_with(
	explicit: Option<&str>,
	product_name: &str,
	os: UserDataOs,
	env: &UserDataPathEnv,
) -> PathBuf {
	if let Some(explicit) = explicit {
		return resolve_relative_to_cwd(explicit, os);
	}

	if let Some(portable) = &env.vscode_portable {
		return join_component(resolve_relative_to_cwd(portable, os), "user-data", os);
	}

	if let Some(appdata) = &env.vscode_appdata {
		return join_component(resolve_relative_to_cwd(appdata, os), product_name, os);
	}

	default_user_data_path_with(product_name, os, env)
}

/// Resolves the platform default user dat