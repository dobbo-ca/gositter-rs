resource "a" "b" {
  c = "d-${e.f}-${g[0]}"
  h = <<-EOF
    #!/bin/sh
    echo "${i}" $HOME $$ %% \
    EOF
  j = [for k in l : "${k!" if k != ""]
}
