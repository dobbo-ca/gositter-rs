
STR."Hello There";
STR."Hello \{x + y}";
STR."We(can have \{many(of)} \{them.in} a \{row}";
compPass("""
        StringTemplate result = RAW.\"""
             @\\{}
  ?           \""";}         """`;
