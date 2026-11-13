using DataStructures
using OrderedCollections
using BenchmarkTools
using Random

suite = BenchmarkGroup()

dicttypes = [Dict, RobinDict, SwissDict]
KVtypes = [Int, Float64]
aexps = [4]
for dt in dicttypes
    for aexp in aexps
        for K in KVtypes
            V = K
            Random.seed!(0)
            sample = rand(K, 10^aexp, 2)
            entries = [sample[i, 1]=>sample[i, 2] for i in 1:size(sample, 1)]
            sample_str = string(" 10^$aexp elem $K")
            
            dict = (dt == Dict) ? string("base-Dict") : string(dt)

            # dict_el_type: {Any, Any}
            suite["make "*dict*"()"*sample_str] = @benchmarkable $(dt)($entries)
            suite["pop! "*dict*"()"*sample_str] = @benchmarkable pop!(h) setup=(h=$(dt)($entries))
            suite["empty! "*dict*"()"*sample_str] = @benchmarkable e