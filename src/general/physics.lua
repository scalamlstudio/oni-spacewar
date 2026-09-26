local Physics = {}

Physics.world = love.physics.newWorld(0, 0, true)

function Physics:load()
    Physics.world:setCallbacks(beginContact, endContact, preSolve, postSolve)
end
function Physics:update(dt)
    Physics.world:update(dt)
end

function beginContact(a, b, coll)
    x, y = coll:getNormal()
    local aobj = a:getBody():getUserData()
    local bobj = b:getBody():getUserData()
    aobj:handle({etype = "hit", object = bobj})
    bobj:handle({etype = "hit", object = aobj})
end

function endContact(a, b, coll)
end
function preSolve(a, b, coll)
end
function postSolve(a, b, coll, normalimpulse, tangentimpulse)
end

return Physics