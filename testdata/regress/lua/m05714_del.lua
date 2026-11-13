-- example of for with generator functions

function generatefib (n)
  return coroutine.wrap(function ()
    loca a,b = 1, 1
    while a <= n do
      coroutine.yield(a)
      a, b = b, a+b
    end
  end)
end

for i in generatefib(1000) do print(i) end
