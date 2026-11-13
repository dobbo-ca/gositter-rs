
try catch end
try finally end

try
    sqrt(x)
catch
    sqrt(complex(x, 0))
end

try
    operate_on_file(f)
finally
    close(f)
end

try
    # fallible
catch
    # handle 