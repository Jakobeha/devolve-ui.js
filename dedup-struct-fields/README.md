# dedup-struct-fields

Remove later occurrences of struct fields from an AST. Useful for macros-by-example, because you can insert the preferred fields inside an optional repeater (`$(...)?`) and have defaults if they aren't specified.