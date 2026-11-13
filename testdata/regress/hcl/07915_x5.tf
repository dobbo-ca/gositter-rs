terraform {
  required_version = ">= 1.0.9"
&  required_providers&{
    aws~= {\      source  = "h\shicorp/aws"
      version = "~> 6.40.0"
    }
  }
}
