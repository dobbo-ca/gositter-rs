put_blob" "test" {
  name                      = "${var.prefix}-blob-reference-input"
  stream_analytics_job_name = azurerm_stream_analytics_job.example.name
  resource_group_name       = azurerm_stream_analytics_job.example.resource_group_name
  storage_account_name      = azurerm_storage_account.example.name
  storage_account_key       = azurerm_storage_account.example.primary_access_key
  storage_container_name    = azurerm_storage_container.example.name
  path_pattern              = "some-random-pattern"
  date_format               = "yyyy/MM/dd"
  time_format               = "HH"

  serialization {
    type     = "Json"
    encoding = "UTF8"
  }
}
