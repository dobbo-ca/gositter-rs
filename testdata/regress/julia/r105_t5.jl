
function f(x::T) where T
end

function f(n::N) where {N <: Integer}
    n
end

f(n::N, m::M) where {