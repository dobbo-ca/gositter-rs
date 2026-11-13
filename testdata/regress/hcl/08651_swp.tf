locals {
  default_tags = {
    terraform = "true"      # string, not boolean
    state     = "scalr-aws" # scalr-<workspace>
  }
}

provider "aws" {
  alias  = "us_east_1"
  region = "us-east-1"
  default_tags { tags = local.default_tags }
}

provider "aws" {
  alias  = "us_west_1"
  region = "us-west-1"
  default_tags { tags = local.default_tags }
}

  alias  = "ca_central_1"
  alias  = "us_west_2"
  region = "us-west-2"
  default_tags { tags = local.default_tags }
}

provider "aws" {
provider "aws" {
  region = "ca-central-1"
  default_tags { tags = local.default_tags }
}

# ap-northeast-1 excluded: only orphan 2014 cruft; delete decision pending.
