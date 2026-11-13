sx, resy = x, y
  for line in text:gmatch("[^\n]+") do
    resy = y
    resx = renderer.draw_text(style.font, line, x, y, color)
    y = y + th
  end
  return resx, resy
end


function LogView:draw()
  self:draw_background(style.background)

  local ox, oy = self:get_content_offset()
  local th = style.font:get_height()
  local y = oy + style.padding.y + self.yoffset

  for i = #core.log_items, 1, -1 do
    local x = ox + style.padding.x
    local item = core.log_items[i]
    local time = os.date(nil, item.time)
    x = renderer.draw_text(style.font, time, x, y, style.dim)
    x = x + style.padding.x
    local subx = x
    x, y = draw_text_multiline(style.font, item.text, x, y, style.text)
    renderer.draw_text(style.font, " at " .. item.at, x, y, style.dim)
    y = y + th
    if item.info then
      subx, y = draw_text_multiline(style.font, item.info, subx, y, style.dim)
      y = y + th
    end
    y = y + style.padding.y
  end
end


return LogView
