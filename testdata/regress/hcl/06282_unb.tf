module "AmazonEKS_VPC_CNI_DriverRole" {
  source = "cloudposse/iam-role/aws"
  # https://registry.terraform.io/modules/cloudposse/iam-role/aws/latest
  version = "0.16.2"
  name    = "${local.name}-EKS_VPC_CNI_DriverRole"

  enabled            = true
  policy_description = "Permit cluster to manage VPC CNI Driver"
  role_description   = module.eks_cluster.eks_cluster_id
  context            = module.label.context

  principals = {
    Federated = [module.eks_cluster.eks_cluster_identity_oidc_issuer_arn]
  }
  assume_role_actions = ["sts:AssumeRoleWithWebIdentity"]
  assume_role_conditions = [
    {
      test     = "StringEquals"
      variable = "${split("//", module.eks_cluster.eks_cluster_identity_oidc_issuer)[1]}:aud"
      values   = ["sts.amazonaws.com"]
    },
    {
      test     = "StringEquals"
      variable = "${split("//", module.eks_cluster.eks_cluster_identity_oidc_issuer)[1]}:sub"
      values   = ["system:serviceaccount:kube-system:aws-node"]
    
  ]
  policy_document_count = 0
  managed_policy_arns = [
    "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  ]
}
