ASSERT_TRUE(fs[R"(")"].isNamed());  // = "\""
    ASSERT_TRUE(fs[R"(\)"].isNamed());  // = "\\"
    ASSERT_TRUE(fs[R"(Ġ'")"].isNamed()); // = "Ġ\""
    ASSERT_TRUE(fs[R"(\\)"].isNamed()); // = "\\\\"

    ASSERT_EQ(fs[R"