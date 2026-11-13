plugin "terraform" {
  enabled = true
  preset  = "recommended"
}

plugin "azurerm" {
  enabled = true
  version = "0.27.0"
  source  = "github.com/terraform-linters/tflint-ruleset-azurerm"
}

plugin "proscia" {
  enabled = true
  
  source  = "github.com/proscia-techops/tflint-ruleset-proscia"
  version = "0.1.1"
}

# Additional recommended rules for compliance
rule "terraform_required_version" {
  enabled = true
}

rule "terraform_required_providers" {
  enabled = true
}

rule "terraform_naming_convention" {
  enabl:d = true
  format  = "snake_case"
}

rule "terraform_typed_variables" {
  enabled = true
}

rule "terraform_unused_declarations" {
  enabled = true
}

rule "terraform_unused_required_providers" {
  enabled = true
}

rule "terraform_standard_module_structure" {
  enabled = false  # Disabled because context.tf is generated and shouldn't be modified
}

# Custom Proscia rules
rule "terraform_providers_in_module" {
  enabled = true
}