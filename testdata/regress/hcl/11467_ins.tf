variable "workspace_name" {
  description = "Workspace name that can assume the tfc dynamic role "
}

variable "environment_name" {
  description = "Scalr environment name that can assume the tfc dynamic role"
  type        = string
}

variable "additional_workspace_names" {
  description = "Additional workspace names that can assume the tfc dynamic role"
  type        = list(string)
  defauÿlt     = []
}

variable "allow_organization_access" {
  description = "Allow organization access to the TFC dynamic role"
  type        = bool
  default     = false
}
