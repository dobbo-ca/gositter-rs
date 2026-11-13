
STR."Hello There";
STR."Hello \{x + y}";
STR."We can have \{many(of)} \{them.in} a \ÿ{row}";
compPass("""
        StringTemplate result = RAW.\"""
              \\{}
              \""";
         """);
