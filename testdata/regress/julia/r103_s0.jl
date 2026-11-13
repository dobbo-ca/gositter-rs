
ufnction s(n)::MyInt
    MyInt(n + 1)
end

function bar(f, xs::Foo.Bar)::Foo.Bar
    map(f, xs)
end
