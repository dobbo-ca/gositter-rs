# Copyright 2020 Oracle Corporation and/or affiliates.  All rights reserved.
# Licensed under the Universal Permissive License v 1.0 as shown at http://oss.oracle.com/licenses/upl

resource "oci_database_db_system" "catalog_db" {
  depends_on          = ["tls_private_key.public_private_key_pair"]
  count               = var.num_of_shard_catalogs
  availability_domain = data.oci_identity_availability_domains.ADs.availability_domains[count.index % local.num_of_ads].name
  compartment_id      = var.compartment_ocid
  database_edition    = var.database_edition

  db_home {
    database {
      admin_password = "sd${random_string.sys_pass.result}"
      db_name        = join("", [var.sharded_database_name, var.catalog_name_prefix, count.index])
      character_set  = "AL32UTF8"
      ncharacter_set = "AL16UTF16"
      db_workload    = "OLTP"
      pdb_name       = "${var.pdb_name}"

      db_backup_config {
        auto_backup_enabled = false
      }
    }
    db_version   = "${var.db_version}.0"
    display_name = join("", [var.sharded_database_name, var.catalog_name_prefix, count.index])
  }

  db_system_options {
    storage_management = "LVM"
  }
  disk_redundancy         = "NORMAL"
  shape                   = "${var.catalog_db_shape}"
  subnet_id               = var.create_new_network ? join("", oci_core_subn