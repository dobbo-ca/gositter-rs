
const char *s1 = R"(
  This is a string. It ends with ')' and a quote.
)";

const char *s2 = R"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";

const char *s3 = uR"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";

const cha