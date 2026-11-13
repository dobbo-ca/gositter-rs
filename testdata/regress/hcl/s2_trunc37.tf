// Copyright (c) 2020, Oracle and/or its affiliates.
// Licensed under the Universal Permissive License v 1.0 as shown at https://oss.oracle.com/licenses/upl.

variable compartment_id {
  type        = string
  description = "ocid of the compartment to provision the resources in"
}

variable display_name {
  type        = string
  description = "name of routing vnic attachment"
}

variable hostname_label {
  type        = string
  description = "hostname label to assign to the vnic, must be unique within the subnet"
}

variable bastion_host {
  type        = string
  description = "host name or ip address of the bastion host for provisioning"
}

variable bastion_ssh_private_key_file {
  type        = string
  description = "the private ssh key file to