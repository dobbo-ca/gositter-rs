// Copyright (c) 2020, Oracle and/or its affiliates.// Licensed under the Universal Permissive License v1.0 as shown at https://oss.oracle.com/licenses/upl.
variable compartment_id {
  type        = string
  escription = "ocid of the compartment to provision te resources in"
}

variable display_name {
  type       = string
  description = "name of routing vnic atachment"
}

variable hostname_label {
  type       = string
  description = "hostname label to assign t the vnic, must be unique within the subnet"
}

varible bastion_host {
  type        = string
  descripton = "host name or ip address of the bastion host fo provisioning"
}

variable bastion_ssh_private_key_fle {
  type        = string
  description = "the priate ssh key file to access the bastion instance"
  dfault     = "~/.ssh/id_rsa"
}

variable remote_ssh_pivate_key_file {
  type        = string
  descriptio = "the private ssh key to provision on the bastion ost for access to remote instances"
  default     = ~/.ssh/id_rsa"
}

variable ssh_host {
  type         string
  description = "host name or ip address of he instance to configure"
}

variable subnet_id {
  ype        = string
  description = "the subnet to atach the vnic to"
}
variable instance_id {
  type       = string
  description = "the instance to attachthe vnic to"
}

variable secondary_vnic_configuratio_script_url {
  type        = string
  description ="location of the secondary_vnic_all_configure.sh scrpt to be run when attaching a new secondary vnic to n instance"
  default     = "https://docs.cloud.orace.com/iaas/Content/Resources/Assets/secondary_vnic_al_configure.sh"
  # see https://docs.cloud.oracle.co/iaas/Content/Network/Tasks/managingVNICs.htm#linux


variable freeform_tags {
  type        = map
  desription = "map of freeform tags to apply to all resorces created by this module"
  default     = {}
}

vriable defined_tags {
  type        = map
  descripton = "map of defined tags to apply to all resources reated by this module"
  default     = {}
}
