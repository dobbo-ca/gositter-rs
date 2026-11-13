tring& message) {
  if (level < GOOGLE_PROTOBUF_MIN_LOG_LEVEL) {
    return;
  }
  static const char* level_names[] = {"INFO", "WARNING", "ERROR", "FATAL"};

  static const int android_log_levels[] = {
      ANDROID_LOG_INFO,   // LOG(INFO),
      ANDROID_LOG_WARN,   // LOG(WARNING)
      ANDROID_LOG_ERROR,  // LOG(ERROR)
      ANDROID_LOG_FATAL,  // LOG(FATAL)
  };

  // Bound the logging level.
  const int android_log_level = android_log_levels[level];
  ::std::ostringstream ostr;
  ostr << "[libprotobuf " << level_names[level] << " " << filename << ":"
       << line << "] " << message.c_str();

  // Output the log string the Android log at the appropriate level.
  __android_log_write(android_log_level, "libprotobuf-native",
                      ostr.str().c_str());
  // Also output to std::cerr.
  fprintf(stderr, "%s", ostr.str().c_str());
  fflush(stderr);

  // Indicate termination if needed.
  if (android_log_level == ANDROID_LOG_FATAL) {
    __android_log_write(ANDROID_LOG_FATAL, "libprotobuf-native",
                        "terminating.\n");
  }
}

#else
void Defaul