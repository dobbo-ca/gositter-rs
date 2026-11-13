ing
  description = "the private ssh key to provision on the bastion host for access to remote instances"
  default     = "~/.ssh/id_rsa"
}

variable ssh_host {
  type        = string
  description = "host name or ip address of the instance to configure"
}

variable subnet_id {
  type        = string
  description = "the subnet to attach the vnic to"
}
variable instance_id {
  type        = string
  description = "the instance to attach the vnic to"
}

variable secondary_vnic_configuration_script_url {
  type        = string
  description = "location of the secondary_vnic_all_configure.sh script to be run when attaching a new secondary vnic to an instance"
  default     = "https://docs.cloud.oracle.com/iaas/Content/Resources/Assets/secondary_vnic_all_configure.sh"
  # see https://docs.cloud.oracle.com/iaas/Content/Network/Tasks/managingVNICs.htm#linux
}

variable freeform_tags {
  type        = map
  description = "map of freeform tags to apply to all resources created by this module"
  default     = {}
}

variable defined_tags {
  type        = map
  description = "map of defined tags to apply to all resources created by this module"
  default     = {}
}
