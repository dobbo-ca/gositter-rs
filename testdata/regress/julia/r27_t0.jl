# Binary heap

struct MutableBinaryHeapNode{T}
    value::T
    handle::Int
end

#################################################
#
#   core implementation
#
#################################################

function _heap_bubble_up!(ord::Ordering,
    nodes::Vector{MutableBinaryHeapNode{T}}, nodemap::Vector{Int}, nd_id::Int) where T

    @inbounds nd = nodes[nd_id]
    v::T = nd.value

    swapped = true  # whether swap happens at last step
    i = nd_id

    while swapped && i > 1  # nd is not root
        p = i >> 1
        @inbounds nd_p = nodes[p]

        if Base.lt(ord, v, nd_p.value)
            # move parent downward
            @inboun