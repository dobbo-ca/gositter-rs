#A counter type

"""
    Accumulator{T, V}

A accumulator is a data structure that maintains an accumulated total for each key.
The particular case where those totals are integers is a counter.
"""
struct Accumulator{T, V} <: AbstractDict{T, V}
    map::Dict{T, V}
end

## constructors

Accumulator{T, V}() where {T, V} = Accumulator{T, V}(Dict{T, V}())
Accumulator(map::AbstractDict) = Accumulator(Dict(map))
function Accumulator(p::Pair, ps::Pair...)
    a = Accumulator(Dict(p))
    for (k,v) in ps
        inc!(a, k, v)
    end
    a
end

counter(T::Type) = Accumulator{T, Int}()
counter(dct::AbstractDict{T, V}) where {T, V<:Integer} = Accumulator{T, V}(Dict(dct))

"""
    counter(seq)

Returns an `Accumulator` object containing the elements from `seq`.
"""
function counter(seq)
    ct = counter(eltype_for_accumulator(seq))
    for x in seq
        inc!(ct, x)
    end
    return ct
end

eltype_for_accumulator(seq::T) where T = eltype(T)
function eltype_for_accumulator(seq::Base.Generator)
    Base.@default_eltype(seq)
end


Base.copy(ct::Accumulator) = Accumulator(copy(ct.map))

Base.length(a::Accumulator) = length(a.map)

## retrieval

Base.get(ct::Accumulator, x, default) = get(ct.map, x, default)
# need to allow user specified default in order to
# correctly 