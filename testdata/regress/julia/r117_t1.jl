
$foo
$obj.field
$(obj.field)
$f(x)
$f[1, 2]
$"foo"

using $Package: $(name)

# Similar definitions in Gadfly/src/varset.jl
mutable struct $(name)
  $(vars...)
end
function $(name)($(parameters_expr))
    $(name)($(pars