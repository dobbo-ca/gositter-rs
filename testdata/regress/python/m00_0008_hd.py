XDG_CONFIG_HOME" not in os.environ


def test_global_uninstall_is_captured_by_sandbox(tmp_path, tmp_path_factory):
    """Global skill deletes land inside the sandbox home, never the real one.

    Since #2215, `claude_uninstall(project_dir)` is project-scoped and must NOT
    touch the global tree; the global delete now requires either a bare call or
    an explicit `remove_user_skill=True`. Both scopes are exercised here so the
    sandbox (#2168) is still proven to capture the global delete.
    """
    skill = Path.home() / ".claude" / "skills" / "graphify" / "SKILL.md"
    skill.parent.mkdir(parents=True)
    skill.write_text("# graphify skill (sandbox copy)\n", encoding="utf-8")

    project_dir = tmp_path / "some-project"
    project_dir.mkdir()

    # Project-scoped call: the global tree in the sandbox home must survive.
    claude_uninstall(project_dir)
    assert skill.exists(), "project-scoped uninstall deleted the global skill (#2215 trap)"

    # Explicit global opt-in: the delete happens, and lands in the sandbox.
    claude_uninstall(project_dir, remove_user_skill=True)
    assert not skill.exists(), "global skill delete was not captured by the sandbox"
    # And the sandbox home itself is still inside pytest's tmp area.
    assert Path.home().is_relative_to(tmp_path_factory.getbasetemp())
