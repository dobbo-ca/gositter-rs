local core = require "core"
local style = require "core.style"
local View = require "core.view"


local LogView = View:extend()


function LogView:new()
  LogView.super.new(self)
  self.last_item = core.log_items[#core.log_items]
  self.scrollable = true
  self.yoffset = 0
end


function LogView:get_name()
  return "Log"
end


function LogView:update()
  local item = core.log_items[#core.log_items]
  if self.last_item ~= item then
    self.last_item = item
    self.scroll.to.y = 0
    self.yoffset = -(style.font:get_height() + style.padding.y)
  end

  self:move_towards("yoffset", 0)

  LogView.super.update(self)
end

