em) == "table" then
      color = item
    else
      x = draw_fn(font, color, item, nil, x, y, 0, self.size.y)
    end
  end

  return x
end


local function text_width(font, _, text, _, x)
  return x + font:get_width(text)
end


function StatusView:draw_items(items, right_align, yoffset)
  local x, y = self:get_content_offset()
  y = y + (yoffset or 0)
  if right_align then
    local w = draw_items(self, items, 0, 0, text_width)
    x = x + self.size.x - w - style.padding.x
    draw_items(self, items, x, y, common.draw_text)
  else
    x = x + style.padding.x
    draw_items(self, items, x, y, common.draw_text)
  end
end


function StatusView:get_items()
  if getmetatable(core.active_view) == DocView then
    local dv = core.active_view
    local line, col = dv.doc:get_selection()
    local dirty = dv.doc:is_dirty()

    return {
      dirty and style.accent or style.text, style.icon_font, "f",
      style.dim, style.font, self.separator2, style.text,
      dv.doc.filename and style.text or style.dim, dv.doc:get_name(),
      style.text,
      self.separator,
      "line: ", line,
      self.separator,
      col > config.line_limit and style.accent or style.text, "col: ", col,
      style.text,
      self.separator,
      string.format("%d%%", line / #dv.doc.lines * 100),
    }, {
      style.icon_font, "g",
      style.font, style.dim, self.separator2, style.text,
      #dv.doc.lines, " lines",
      self.separator,
      dv.doc.crlf and "CRLF" or "LF"
    }
  end

  return {}, {
    style.icon_font, "g",
    style.font, style.dim, self.separator2,
    #core.docs, style.text, " / ",
    #core.project_files, " files"
  }
end


function StatusView:draw()
  self:draw_background(style.background2)

  if self.message then
    self:draw_items(self.message, false, self.size.y)
  end

  local left, right = self:get_items()
  self:draw_items(left)
  self:draw_items(right, true)
end


return StatusView
