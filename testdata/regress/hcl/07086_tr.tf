# IAM role for cert-manager to perform DNS-01 ACME challenges via Route53.
#
# Bound to the `cert-manager/cert-manager` ServiceAccount via an EKS Pod
# Identity association. The eks-p