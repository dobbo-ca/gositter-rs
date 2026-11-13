
function f end

function nop() end

function I(x) x end

function Base.rand(n::MyInt)
    return 4
end

function Γ(z)
    gamma"(z)
end

function ⊕(x, y)
    x + y
end

function fix2(f, x)
    return function(y)
        f(x, y)
    end
end

function (foo::Foo)()
end
