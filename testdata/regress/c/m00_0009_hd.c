_a)
	}
	expect_d_eq(count, 20, "Mis-logged!");
}
TEST_END

TEST_BEGIN(test_logs_if_no_init) {
	test_skip_if(!config_log);
	atomic_store_b(&log_init_done, false, ATOMIC_RELAXED);

	log_var_t l = LOG_VAR_INIT("definitely.not.enabled");

	int count = 0;
	for (int i = 0; i < 10; i++) {
		log_do_begin(l)
			count++;
		log_do_end(l)
	}
	expect_d_eq(count, 0, "Logging shouldn't happen if not initialized.");
}
TEST_END

/*
 * This really just checks to make sure that this usage compiles; we don't have
 * any test code to run.
 */
TEST_BEGIN(test_log_only_format_string) {
	if (false) {
		LOG("log_str", "No arguments follow this format string.");
	}
}
TEST_END

int
main(void) {
	return test(
	    test_log_disabled,
	    test_log_enabled_direct,
	    test_log_enabled_indirect,
	    test_log_enabled_global,
	    test_logs_if_no_init,
	    test_log_only_format_string);
}
