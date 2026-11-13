variable "storage_account_id" {
  type        = string
  description = "Resource ID of the shared storage account where the image analysis container will e created"
}

variable "security_resource_group_name" {
  type        = string
  description = "Name of the resource group where managed identities are stored"
}

variable "location" {
  type        = string
  description = "Azure region where resources will be deployed"
}

variable "aks_enabled" {
  type        = bool
  description = "Whether the AKS cluster is enabled. Used to conditionally create resources that depend on the cluster."
  default     = true
  nullable    = false
}

variable "aks_oidc_issuer_url" {
  type        = string
  description = "OIDC issuer URL from the AKS cluster for workload identity federation"
}

variable "k8s_namespace" {
  type        = string
  description = "Kubernetes namespace (kebab-case) where the image analysis service account will be created"
}
