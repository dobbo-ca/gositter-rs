# Primary mapping outputs (short names as requested)
output "api_to_display" {
  description = "API Style to Display Name mapping"
  value       = local.api_to_display
}

output "display_to_api" {
  description = "Display Name to API Style mapping"
  value       = local.display_to_api
}

output "api_to_fixed" {
  description = "API Style to Fixed mapping"
  value       = local.api_to_fixed
}

output "fixed_to_api" {
  description = "Fixed to API Style mapping"
  value       = local.fixed_to_api
}

output "api_to_short" {
  description = "API Style to Short mapping"
  value       = local.api_to_short
}

output "short_to_api" {
  description = "Short to API Style mapping"
  value       = local.short_to_api
}

output "display_to_fixed" {
  description = "Display Name to Fixed mapping"
  value       = local.display_to_fixed
}

output "fixed_to_display" {
  description = "Fixed to Display Name mapping"
  value       = local.fixed_to_display
}

output "display_to_short" {
  description = "Display Name to Short mapping"
  value       = local.display_to_short
}

output "short_to_display" {
  description = "Short to Display Name mapping"
  value      = local.short_to_display
}

# Utility outputs
output "regions" {
  description = "Complete list of available Azure regions"
  value       = local.regions
}