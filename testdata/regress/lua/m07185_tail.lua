r line = line, #doc.lines do
    local line_text = doc.lines[line]
    if opt.no_case then
      line_text = line_text:lower()
    end
    local s, e = line_text:find(text, col, not opt.pattern)
    if s then
      return line, s, line, e + 1
    end
    col = 1
  end

  if opt.wrap then
    opt = { no_case = opt.no_case, pattern = opt.pattern }
    return search.find(doc, 1, 1, text, opt)
  end
end


return search
