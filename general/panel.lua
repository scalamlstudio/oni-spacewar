-- Panel

local Panel = {}

function Panel:new(x, y)
    local panel = {}

    panel.width = love.graphics.getWidth()
    panel.height = love.graphics.getHeight()

    panel.text = "DEFAULT TEXT"
    panel.x = x
    panel.y = y

    function panel:update(newText)
        panel.text = newText
    end

    function panel:add(newText)
        panel.text = panel.text .."\n" .. newText
    end

    function panel:trim()
        if string.len(panel.text) > 100 then -- cleanup when 'text' gets too long
            panel.text = ""
        end
    end

    function panel:draw()
        love.graphics.setColor(0, 1, 0)
        love.graphics.print(panel.text, panel.x, panel.y)
    end

    return panel
end

return Panel