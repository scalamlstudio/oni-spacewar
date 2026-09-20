-- Camera

local Camera = {}

Camera.x = - love.graphics.getWidth() / 2
Camera.y = - love.graphics.getHeight() / 2
Camera.vx = 0
Camera.vy = 0
Camera.sticky = 0.3
Camera.scaleX = 1
Camera.scaleY = 1
Camera.rotation = 0
Camera.vibration = 0

function Camera:set()
    love.graphics.push()
    love.graphics.rotate(-self.rotation)
    love.graphics.scale(1 / self.scaleX, 1 / self.scaleY)
    love.graphics.translate(-self.x, -self.y)
end

function Camera:unset()
    love.graphics.pop()
end

function Camera:move(dx, dy)
    self.x = self.x + (dx or 0)
    self.y = self.y + (dy or 0)
end

function Camera:rotate(dr)
    self.rotation = self.rotation + dr
end

function Camera:scale(sx, sy)
    sx = sx or 1
    self.scaleX = self.scaleX * sx
    self.scaleY = self.scaleY * (sy or sx)
end

function Camera:setPosition(x, y)
    self.x = x or self.x
    self.y = y or self.y
end

function Camera:setScale(sx, sy)
    self.scaleX = sx or self.scaleX
    self.scaleY = sy or self.scaleY
end

function Camera:follow(world, obj)
    Camera.x = Camera.x + Camera.vx + Camera.vibration * math.random(-5, 5)
    Camera.y = Camera.y + Camera.vy + Camera.vibration * math.random(-5, 5)
    Camera.vx = Camera.vx * Camera.sticky + (obj.x - world.w / 2 - Camera.x) * (1 - Camera.sticky)
    Camera.vy = Camera.vy * Camera.sticky + (obj.y - world.h / 2 - Camera.y) * (1 - Camera.sticky)
end

function Camera:setObj(world, obj)
    Camera:setPosition(obj.x - world.w / 2, obj.y - world.h / 2)
end

function Camera:vibrate(vibration)
    Camera.vibration = vibration
end

function Camera:update(dt)
    Camera.vibration = math.max(0, Camera.vibration - dt)
end

return Camera