  curl_mprintf("%d\n", status);
  return status ? TEST_ERR_FAILURE : CURLE_OK;

test_cleanup:

  curl_easy_cleanup(curl);
  curl_global_cleanup();

  return result;
}
