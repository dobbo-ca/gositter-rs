PTTYPE_LONG, 	85),

  /* type of the file keeping your SSL-certificate ("DER", "PEM", "ENG") */
  CURLOPT(CURLOPT_SSLCERTTYPE, CURLOPTTYPE_STRINGPOINT, 86),

  /* name of the file keeping your private SSL-key */
  CURLOPT(CURLOPT_SSLKEY, CURLOPTTYPE_STRINGPOINT, 87),

  /* type of the file keeping your private SSL-key ("DER", "PEM", "ENG") */
  CURLOPT(CURLOPT_SSLKEYTYPE, CURLOPTTYPE_STRINGPOINT, 88),

  /* crypto engine for the SSL-sub system */
  CURLOPT(CURLOPT_SSLENGINE, 