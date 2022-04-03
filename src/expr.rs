use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Expr, Member, Token,
};

//essentially a syn::ExprStruct but with no name, attrs or rest
pub struct ExprUnnamedStruct<T: Parse + ToTokens> {
    pub brace_token: token::Brace,
    pub fields: Punctuated<FieldValue<T>, Token![,]>,
}

impl<T: Parse + ToTokens> Parse for ExprUnnamedStruct<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let brace_token = braced!(content in input);

        let mut fields = Punctuated::new();
        while !content.is_empty() {
            if content.peek(Token![..]) {
                return Ok(ExprUnnamedStruct {
                    brace_token,
                    fields,
                });
            }

            fields.push(content.parse()?);
            if content.is_empty() {
                break;
            }
            let punct: Token![,] = content.parse()?;
            fields.push_punct(punct);
        }

        Ok(ExprUnnamedStruct {
            brace_token,
            fields,
        })
    }
}

impl<T: Parse + ToTokens> ToTokens for ExprUnnamedStruct<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.brace_token.surround(tokens, |tokens| {
            self.fields.to_tokens(tokens);
        });
    }
}

//change FieldValue too so we can extend the field values with other structures
pub struct FieldValue<T: Parse + ToTokens> {
    pub member: Member,
    pub colon_token: Option<Token![:]>,
    pub expr: T,
}

impl<T: Parse + ToTokens> Parse for FieldValue<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let member: Member = input.parse()?;

        if input.peek(Token![:]) || !matches!(member, Member::Named(_)) {
            let colon_token: Token![:] = input.parse()?;
            let value: T = input.parse()?;

            Ok(FieldValue {
                member,
                colon_token: Some(colon_token),
                expr: value,
            })
        } else {
            unreachable!()
        }
    }
}

impl<T: Parse + ToTokens> ToTokens for FieldValue<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.member.to_tokens(tokens);
        if let Some(colon_token) = &self.colon_token {
            colon_token.to_tokens(tokens);
            self.expr.to_tokens(tokens);
        }
    }
}

//our own enumeration of possible values for the field values
pub enum CustomExpr {
    Expr(Box<Expr>),
    ExprUnnamedStruct(ExprUnnamedStruct<CustomExpr>),
}

impl Parse for CustomExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Brace) {
            let obj = input.parse::<ExprUnnamedStruct<CustomExpr>>()?;
            Ok(CustomExpr::ExprUnnamedStruct(obj))
        } else {
            let expr = input.parse::<Expr>()?;
            Ok(CustomExpr::Expr(Box::new(expr)))
        }
    }
}

impl ToTokens for CustomExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            CustomExpr::Expr(value) => value.to_tokens(tokens),
            CustomExpr::ExprUnnamedStruct(value) => value.to_tokens(tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse::Parser, Attribute};

    #[test]
    fn test_attribute() {
        let attrs = Parser::parse_str(
            Attribute::parse_outer,
            "#[blah({ name: \"MyVal\", age: 33, props: [1,2,3]})]",
        )
        .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args::<ExprUnnamedStruct<CustomExpr>>()
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(
            elem_str,
            "{ name : \"MyVal\" , age : 33 , props : [1 , 2 , 3] }"
        );
    }

    #[test]
    fn test_attribute_nested() {
        let attrs = Parser::parse_str(
            Attribute::parse_outer,
            "#[blah({ name: \"MyVal\", age: 33, props: [1,2,3], other: { name: \"ok\" }})]",
        )
        .expect("attribute");
        let elem = attrs
            .first()
            .unwrap()
            .parse_args::<ExprUnnamedStruct<CustomExpr>>()
            .expect("Could not parse the args");
        let elem_str = elem.to_token_stream().to_string();

        assert_eq!(
            elem_str,
            "{ name : \"MyVal\" , age : 33 , props : [1 , 2 , 3] , other : { name : \"ok\" } }"
        );
    }
}
