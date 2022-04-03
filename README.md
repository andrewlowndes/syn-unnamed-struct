# syn_unnamed_struct
Parse and convert structs with no name to tokens. For usage in attribute macro arguments in place of Meta attributes to allow more structured data to be used (nested objects).

## Usage in derive macro definition
```rust
use syn_unnamed_struct::Meta;

#[proc_macro_derive(CustomMacro, attributes(customMacro))]
pub fn derive(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens);
    
    input.attrs.map(|attr| {
        let obj: Meta = attr.parse().expect("Coult not parse attribute");
        
        //can now interact and extract the properties from the Meta enum
        //...
    });
}
```

### Example macro usage
```rust
#[derive(CustomMacro)]
#[customMacro(name="something", other={ entry1: "val1", entry2: "val2" })]
struct MyStruct {
    //...
}
```

## Supported attributes

- **Unnamed structs**
```rust
#[customMacro({ prop1: 123, prop2: 245 })]
```

- **Nested unnamed structs**
```rust
#[customMacro({ prop1: 123, prop2: { prop2a: 123, prop2b: 245 } })]
```

- **Unnamed struct in Meta value**
```rust
#[customMacro(prop1=123, prop2={ prop2a: 123, prop2b: 245 })]
```

- **Unnamed Meta list**
```rust
#[customMacro(prop1, prop2, (prop3a=123, prop3b=245)))]
```

- **Nested unnamed Meta lists**
```rust
#[customMacro(prop1=123, prop2=(prop2a=123, prop2b=245)))]
```

## Notes
- Cannot use [darling](https://docs.rs/darling/latest/darling/index.html) since it is entwined with the syn Meta structs
