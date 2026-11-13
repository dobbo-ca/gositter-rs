@testset "SplayTree" begin
    @testset "pushing values" begin
        t = SplayTree{Int}()
        for i in 1:100
            push!(t, i)
        end

        @test length(t) == 100

        for i in 1:100
            @test haskey(t, i)
        end

        for i = 101:200
            @test !haskey(t, i)
        end
    end

    @testset "deleting values" begin
        t = SplayTree{Int}()
        for i in 1:100
            push!(t, i)
        end
        for i in 1:2:100
            delete!(t, i)
        en