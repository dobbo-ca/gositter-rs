# Benchmark on heaps

using DataStructures

# benchmark function

function benchmark_heap(title::String, h::AbstractHeap, xs::Vector{Float64})
    @assert isempty(h)

    # warming
    push!(h, 0.5)
    pop!(h)

    # bench
    n = length(xs)

    t1 = @elapsed for i = 1