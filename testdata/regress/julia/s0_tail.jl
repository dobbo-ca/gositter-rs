($entries); e=($entries)[1, 1])
            suite["find-failure "*dict*"()"*sample_str] = @benchmarkable get(h, e, -1) setup=(h=$(dt)($entries); e=first(pop!(h)))
            # dict_el_type: {K, V}
            suite["make "*dict*"{$K,$V}()"*sample_str] = @benchmarkable $(dt){$K,$V}($entries)
            suite["pop! "*dict*"{$K,$V}()"*sample_str] = @benchmarkable pop!(h) setup=(h=$(dt){$K,$V}($entries))
            suite["empty! "*dict*"{$K,$V}()"*sample_str] = @benchmarkable empty!(h) setup=(h=$(dt){$K,$V}($entries))
            suite["find-success "*dict*"{$K,$V}()"*sample_str] = @benchmarkable get(h, e, -1) setup=(h=$(dt){$K,$V}($entries); e=($entries)[1, 1])
            suite["find-failure "*dict*"{$K,$V}()"*sample_str] = @benchmarkable get(h, e, -1) setup=(h=$(dt){$K,$V}($entries); e=first(pop!(h)))
        end
    end
end 

results = run(suite, verbose=true);

function filter_result(results, op, dt, apow=4)
    dict = (dt == Dict) ? string("base-Dict") : string(dt)
    for tr in keys(filter(k->(occursin(op, first(k)) && occursin(dict, first(k))
                                 && occursin("10^$apow", first(k))) , results))
           println(tr, " memory ", memory(results[tr])/1024, " kB")
           println(tr, " minimum time ", minimum(results[tr]))
       end
end
