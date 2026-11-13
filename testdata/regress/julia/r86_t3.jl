module Geometry

using LinearAlgebra
import Base: show
using Base.Threads
using ..ParentModule

abstract type Shape end

struct Point <: Shape
    x::Float64
    y::Float64
end

mutable struct Circle <: Shape
    center::Point
    radius::Float64
end

function area(c::Circle)
    return pi * c.radius^2
end

function distanc