


# Contributing

## Commit Messages

*TODO*

## Formatting

- Use 4 spaces for indentation.
- Do not exceed 100 characters per line.
- Types should be in `PascalCase`, and functions/variables should be in `snake_case`.
- Delineate code sections 4 newline characters (3 empty lines).
- Files should always end with a newline character.

### Example

```rust
//! Module-level documentation.

#![module_level_attribute]

mod declaration;

use statement;



pub struct ExportedType;

impl ExportedType {
    pub fn new() -> Self {
        todo!()
    }
}



struct LocalType {
    field: u8,
}

impl LocalType { /* TODO */ }

```
