OT
    If app_vpns is not empty, then this output contains the VPC CIDR blocks of the app and app VPNs.
    EOT
  value = concat(
    local.vpn_enabled ? [module.vpn_peering[0].vpn_vpc_cidr, module.vpc_app.vpc_cidr_block] : [],
    local.vpn2_enabled ? [module.vpn_peering2[0].vpn_vpc_cidr] : []
  )
}
