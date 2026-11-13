/*
# Copyright (c) 2018, 2020, Oracle Corporation and/or its affiliates.
# Licensed under the Universal Permissive License v 1.0 as shown at https://oss.oracle.com/licenses/upl.
*/
variable "cluster_kubernetes_version" { default = "v1.11.5" }
variable "cluster_name" { default = "tfTestCluster" }
variable "cluster_options_add_ons_is_kubernetes_dashboard_enabled" { default = true }
variable "cluster_options_add_ons_is_tiller_enabled" { default = true }
variable "cluster_options_kubernetes_network_config_pods_cidr" { default = "10.1.0.0/16" }
variable "cluster_options_kubernetes_network_config_services_cidr" { default = "10.2.0.0/16" }
variable "node_pool_initial_node_labels_key" { default = "key" }
variable "node_pool_initial_node_labels_value" { default = "value" }
variable "node_pool_kubernetes_version" { default = "v1.11.5" }
variable "node_pool_name" { default = "tfTestCluster_workers" }
variable "node_pool_node_image_name" { default = "Oracle-Linux-7.4" }
variable "n