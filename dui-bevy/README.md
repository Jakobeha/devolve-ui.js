# dui-basic: basic .dui sub-format 2D scenes supporting minimal graphics

These `.dui`s are represented in TOML. However they must start with `# basic` so that devolve-ui can identify this sub-format.

## Example

```toml
# dui-basic
# ^ every file must start with this

# interface
# this must match the structure of the DuiInterface you pass to this
# each field's key must match the key in the structure
# each field's value must be:
# - primitive boolean, number, string, ... or tuple, array, vec, or option = type name as string
# - In one of the above = "in:type"
# - Out of one of the above = "out=value", where "value" one of the following
#   - mouse = mouse position (type = (f32, f32))
#   - keys = keys pressed (type = BitArr!(for 256), each index corresponds to the key-code)
#   * remember: any more complicated outputs or control-flow are handled in the devolve-ui code. Outputs are simply the minimum information you need from the user
# - In, Out, InOut of a user-defined type = unsupported
#   * remember: instead of In with a structure, you can write the structure and have each of its fields be an In
# - user-defined-type = table of fields which follows this spec recursively
[interface]
title = "String"
subtitle = "Option<String>"
pos = "in:(f32, f32)"
radius = "in:f32"
keys = "out=keys"

# common properties inherited by multiple views
[common.wall]
type = "rect"
color = "blue"

# views AKA text, shapes, etc. each view may have a key but doesn't have to, it's only for the user
[[views]]
key = "left-wall"
bounds = [0, 0, 0.1, 1]
inherits = "wall"

[[views]]
key = "right-wall"
bounds = [0.9, 0, 1, 1]
inherits = "wall"

[[views]]
key = "top-wall"
bounds = [0, 0, 1, 0.05]
inherits = "wall"

[[views]]
key = "bottom-wall"
bounds = [0, 0.8, 1, 1]
inherits = "wall"

[[views]]
key = "ball"
type = "circle"
bounds = ["pos - (radius / 2)", "pos - (radius / 2)", "pos + (radius / 2)", "pos + (radius / 2)"]
color = "red"

[[views]]
type = "rect"
bounds = [0.3, 0.7, 0.35, 0.8]
```
    
