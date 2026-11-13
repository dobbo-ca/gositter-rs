
const char *s1 = R"(
  This is a string. It ends";

const char *s2 = R"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";

const char *s3 = uR"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";

const char *s4 = UR"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";

const char *s5 = u8R"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";

const char *s6 = LR"FOO(
  This is a string. It ends with ')FOO' and a quote.
)FOO";
