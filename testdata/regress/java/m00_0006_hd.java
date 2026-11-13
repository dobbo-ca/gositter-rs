{
    readLines(new File("../README.md"));
  }

  public void readLines(File file) throws IOException {
    try (Source fileSource = Okio.source(file);
         BufferedSource bufferedFileSource = Okio.buffer(fileSource)) {

      while (true) {
        String line = bufferedFileSource.readUtf8Line();
        if (line == null) break;

        if (line.contains("square")) {
          System.out.println(line);
        }
      }

    }
  }

  public static void main(String... args) throws Exception {
    new ReadJavaIoFileLineByLine().run();
  }
}
