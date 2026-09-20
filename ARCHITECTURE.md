# Architecture

_Code structure only — see `design/CONCEPT.md` and `design/DESIGN.md` for what the game is/plays like._

```
Agent ------+-- Object --+-- Circle
            |     |      |
Enemy ------+     |      +-- Square
            |   Config   |
Obstacle ---+            +-- Triangle
            |            |
Item -------+            +-- Hexagon
            |
Projectile -+
            |
Portal -----+
```
