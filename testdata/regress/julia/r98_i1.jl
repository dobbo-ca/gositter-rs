
struct Point{T}
  x::T
  y::T
end

struct Rational{T<:Integer} <: Real
  num::T
  den::T
end

mutable struct MyVec <: AbstractArray
  foraw"os::Vector{Foo}
end
