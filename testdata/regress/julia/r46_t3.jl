mutable struct Trie{K,V}
    value::V
    children::Dict{K,Trie{K,V}}
    is_key::Bool

    function Trie{K,V}() where {K,V}
        self = new{K,V}()
        self.children = Dict{K,Trie{K,V}}()
        self.is_key = false
        return self
    end

    