use crate::{CustomExpr, ExprUnnamedStruct};
use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Lit, LitBool, Path, PathSegment, Token,
};

//replace the Meta objects too so we can nest unnamed structs in normal meta objects
pub enum Meta {
    Path(MetaPath),
    List(MetaList),
    UnnamedList(UnnamedMetaList),
    NameValue(MetaNameValue),
}

impl Parse for Meta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Paren) {
            let lst = input.parse::<UnnamedMetaList>()?;
            Ok(Meta::UnnamedList(lst))
        } else {
            let path = input.parse::<MetaPath>()?;
            parse_meta_after_path(path, input)
        }
    }
}

impl ToTokens for Meta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Meta::Path(value) => value.to_tokens(tokens),
            Meta::List(value) => value.to_tokens(tokens),
            Meta::UnnamedList(value) => value.to_tokens(tokens),
            Meta::NameValue(value) => value.to_tokens(tokens),
        }
    }
}

pub struct MetaPath(Path);

impl MetaPath {
    pub fn get_inner(&self) -> &Path {
        &self.0
    }
}

impl Parse for MetaPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(MetaPath(Path {
            leading_colon: input.parse()?,
            segments: {
                let mut segments = Punctuated::new();
                while input.peek(Ident::peek_any) {
                    let ident = Ident::parse_any(input)?;
                    segments.push_value(PathSegment::from(ident));
                    if !input.peek(Token![::]) {
                        break;
                    }
                    let punct = input.parse()?;
                    segments.push_punct(punct);
                }
                if segments.is_empty() {
                    return Err(input.error("expected path"));
                } else if segments.trailing_punct() {
                    return Err(input.error("expected path segment"));
                }
                segments
            },
        }))
    }
}

impl ToTokens for MetaPath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

pub struct UnnamedMetaList {
    pub paren_token: token::Paren,
    pub nested: Punctuated<NestedMeta, Token![,]>,
}

impl Parse for UnnamedMetaList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = parenthesized!(content in input);

        Ok(UnnamedMetaList {
            paren_token,
            nested: content.parse_terminated(NestedMeta::parse)?,
        })
    }
}

impl ToTokens for UnnamedMetaList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.paren_token.surround(tokens, |tokens| {
            self.nested.to_tokens(tokens);
        });
    }
}

pub struct MetaList {
    pub path: MetaPath,
    pub paren_token: token::Paren,
    pub nested: Punctuated<NestedMeta, Token![,]>,
}

impl Parse for MetaList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<MetaPath>()?;
        parse_meta_list_after_path(path, input)
    }
}

impl ToTokens for MetaList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.paren_token.surround(tokens, |tokens| {
            self.nested.to_tokens(tokens);
        });
    }
}

//our own enumeration of possible values for the field values
pub enum MetaValue {
    Lit(Lit),
    ExprUnnamedStruct(ExprUnnamedStruct<CustomExpr>),
    UnnamedMetaList(UnnamedMetaList),
}

impl Parse for MetaValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Paren) {
            let lst = input.parse::<UnnamedMetaList>()?;
            Ok(MetaValue::UnnamedMetaList(lst))
        } else if input.peek(token::Brace) {
            let obj = <ExprUnnamedStruct<CustomExpr>>::parse(input)?;
            Ok(MetaValue::ExprUnnamedStruct(obj))
        } else {
            let expr = Lit::parse(input)?;
            Ok(MetaValue::Lit(expr))
        }
    }
}

impl ToTokens for MetaValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            MetaValue::Lit(value) => value.to_tokens(tokens),
            MetaValue::ExprUnnamedStruct(value) => value.to_tokens(tokens),
            MetaValue::UnnamedMetaList(value) => value.to_tokens(tokens),
        }
    }
}

pub struct MetaNameValue {
    pub path: MetaPath,
    pub eq_token: Token![=],
    pub value: MetaValue,
}

impl Parse for MetaNameValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse::<MetaPath>()?;
        parse_meta_name_value_after_path(path, input)
    }
}
impl ToTokens for MetaNameValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.eq_token.to_tokens(tokens);
        self.value.to_tokens(tokens);
    }
}

pub enum NestedMeta {
    Meta(Meta),
    Lit(Lit),
    Expr(CustomExpr),
}

impl Parse for NestedMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Lit) && !(input.peek(LitBool) && input.peek2(Token![=])) {
            input.parse().map(NestedMeta::Lit)
        } else if input.peek(Ident::peek_any)
            || input.peek(Token![::]) && input.peek3(Ident::peek_any)
        {
            input.parse().map(NestedMeta::Meta)
        } else {
            Err(input.error("expected identifier or literal"))
        }
    }
}

impl ToTokens for NestedMeta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            NestedMeta::Meta(value) => value.to_tokens(tokens),
            NestedMeta::Expr(value) => value.to_tokens(tokens),
            NestedMeta::Lit(value) => value.to_tokens(tokens),
        }
    }
}

pub fn parse_meta_after_path(path: MetaPath, input: ParseStream) -> syn::Result<Meta> {
    if input.peek(token::Paren) {
        parse_meta_list_after_path(path, input).map(Meta::List)
    } else if input.peek(Token![=]) {
        parse_meta_name_value_after_path(path, input).map(Meta::NameValue)
    } else {
        Ok(Meta::Path(path))
    }
}

fn parse_meta_list_after_path(path: MetaPath, input: ParseStream) -> syn::Result<MetaList> {
    let content;
    let paren_token = parenthesized!(content in input);
    Ok(MetaList {
        path,
        paren_token,
        nested: content.parse_terminated(NestedMeta::parse)?,
    })
}

fn parse_meta_name_value_after_path(
    path: MetaPath,
    input: ParseStream,
) -> syn::Result<MetaNameValue> {
    Ok(MetaNameValue {
        path,
        eq_token: input.parse()?,
        value: input.parse()?,
    })
}

#[cfg(test)]
mod tests {
    use syn::{parse::Parser, Attribute};

    use super::*;

    #[test]
    fn test_meta() {
        let attrs = Parser::parse_str(Attribute::parse_outer, "#[blah(name=\"MyVal\", active)]")
            .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args_with(<Punctuated<Meta, Token![,]>>::parse_terminated)
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(elem_str, "name = \"MyVal\" , active");
    }

    #[test]
    fn test_meta_list() {
        let attrs = Parser::parse_str(Attribute::parse_outer, "#[blah(a, b, more(one=1, two=2))]")
            .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args_with(<Punctuated<Meta, Token![,]>>::parse_terminated)
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(elem_str, "a , b , more (one = 1 , two = 2)");
    }

    #[test]
    fn test_meta_unnamed_struct() {
        let attrs = Parser::parse_str(
            Attribute::parse_outer,
            "#[blah(name=\"MyVal\", age=33, other={name: \"ok\"})]",
        )
        .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args_with(<Punctuated<Meta, Token![,]>>::parse_terminated)
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(
            elem_str,
            "name = \"MyVal\" , age = 33 , other = { name : \"ok\" }"
        );
    }

    #[test]
    fn test_meta_unnamed_list() {
        let attrs = Parser::parse_str(Attribute::parse_outer, "#[blah(a, b, (one=1, two=2))]")
            .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args_with(<Punctuated<Meta, Token![,]>>::parse_terminated)
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(elem_str, "a , b , (one = 1 , two = 2)");
    }

    #[test]
    fn test_meta_unnamed_nested_list() {
        let attrs = Parser::parse_str(Attribute::parse_outer, "#[blah(a, b, c = (one=1, two=2))]")
            .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args_with(<Punctuated<Meta, Token![,]>>::parse_terminated)
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(elem_str, "a , b , c = (one = 1 , two = 2)");
    }
}
