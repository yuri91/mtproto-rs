#![recursion_limit = "128"]

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate quote;

use std::env;
use std::fs::File;
use std::io::{Write,Read};
use std::path::Path;

// Helper structs to read the json into rust data types
// NOTE: `type` is a keyword, so it is replaced by `kind`
mod schema {
    use super::serde_json;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Param {
        pub name: String,
        #[serde(rename="type")]
        pub kind: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Constructor {
        pub id: i32,
        pub predicate: String,
        pub params: Vec<Param>,
        #[serde(rename="type")]
        pub kind: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Method {
        pub id: i32,
        pub method: String,
        pub params: Vec<Param>,
        #[serde(rename="type")]
        pub kind: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Schema {
        pub constructors: Vec<Constructor>,
        pub methods: Vec<Method>,
    }

    impl Schema {
        pub fn new(s: &str) -> Schema {
            serde_json::from_str(&s).unwrap()
        }
    }
}

// Helper structs to view the schema in a more useful layout
// NOTE: `type` is a keyword, so it is replaced by `kind`
mod ast {
    use super::schema;
    use super::quote;
    use std::collections::HashMap;


    #[derive(Clone, Debug, Default)]
    pub struct Namespace {
        pub types: HashMap<String,Type>
    }
    #[derive(Clone, Debug, Default)]
    pub struct Type {
        pub constructors: HashMap<String,Constructor>
    }
    #[derive(Clone, Debug)]
    pub struct Constructor {
        pub id: i32,
        pub fields: HashMap<String,Field>
    }
    #[derive(Clone, Debug)]
    pub struct Field {
        pub kind: String
    }
    #[derive(Clone, Debug)]
    pub struct Ast {
        pub namespaces: HashMap<String,Namespace>
    }
    mod utils {
        use super::quote;
        pub fn to_uppercase(s: &str) -> String {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
            }
        }
        pub fn split_type(ns_ty:&str) -> (String,String) {
            let mut split = ns_ty.split(".");
            let n1 = split.next();
            let n2 = split.next();
            if n2.is_some() {
                (n1.unwrap().into(),n2.unwrap().into())
            } else {
                ("".into(),n1.unwrap().into())
            }
        }
        // We change the name of the basic types and of the `Vector` type
        // directly to the correspondig rust types.
        // Constructor names are converted to CamelCase and the namespace is
        // indicated with `::` instead of `.`
        fn type_str(ty_name: &str) -> String {
            match ty_name {
                "int" => "i32".to_string(),
                "long" => "i64".to_string(),
                "double" => "f64".to_string(),
                "bytes" => "Vec<u8>".to_string(),
                "string" => "String".to_string(),
                _ => {
                    if let Some(start) = ty_name.find('<') {
                        let end = ty_name.find('>').unwrap();
                        let inner = &ty_name[start+1..end];
                        "Vec<".to_string()+&type_str(inner)+">"
                    } else {
                        let (ns,n) = split_type(ty_name);
                        if ns != "" {
                            "::".to_string() + &ns + "::" + &to_uppercase(&n)
                        } else {
                            "::".to_string() + &to_uppercase(&n)
                        }

                    }
                }
            }
        }
        pub fn type_ident(ty_name: &str) -> quote::Ident {
            quote::Ident::new(type_str(ty_name))
        }
        pub fn field_ident(fi_name: &str) -> quote::Ident {
            let norm_name = match fi_name {
                "type" => "kind".to_string(),
                _ => fi_name.to_string()
            };
            quote::Ident::new(norm_name)
        }
        pub fn constructor_ident(cs_name: &str) -> quote::Ident {
            quote::Ident::new(to_uppercase(cs_name))
        }
    }
    impl Ast {
        pub fn new(schema: &schema::Schema) -> Ast{
            let mut namespaces = HashMap::new();
            for c in &schema.constructors {
                if &c.predicate == "vector" ||
                   &c.predicate == "peerSettings" ||
                   &c.predicate == "true" {
                    continue;
                }

                let (name_ns,name) = utils::split_type(&c.predicate);
                let (kind_ns, kind) = utils::split_type(&c.kind);
                assert_eq!(name_ns,kind_ns);

                let namespace = namespaces.entry(kind_ns)
                    .or_insert(Namespace::default());
                let ty = namespace.types.entry(kind)
                    .or_insert(Type::default());
                let old = ty.constructors.insert(name,Constructor {
                    id: c.id,
                    fields: c.params.iter().map(|p| {
                        (p.name.clone(),Field{kind: p.kind.clone()})
                    }).collect()
                });
                assert!(old.is_none());
            }
            Ast {
                namespaces: namespaces
            }
        }
        pub fn compile(&self) -> quote::Tokens {
            let mut tokens = quote::Tokens::new();
            for (ns_name,ns) in &self.namespaces {
                let ns_ident = quote::Ident::new(ns_name.to_string());
                let mut tokens_ty = quote::Tokens::new();
                for (ty_name,ty) in &ns.types {
                    let ty_ident = quote::Ident::new(ty_name.to_string());
                    let mut tokens_cs = quote::Tokens::new();
                    let mut tokens_cs_ser = quote::Tokens::new();
                    let mut tokens_cs_deser = quote::Tokens::new();
                    for (cs_name,cs) in &ty.constructors {
                        let cs_ident = utils::constructor_ident(cs_name);
                        let (field_idents,field_types) : (Vec<_>,Vec<_>) = cs.fields.iter().map(|(k,ty)|{
                            (utils::field_ident(k),
                            quote::Ident::new(utils::type_ident(ty.kind.as_str())))
                        }).unzip();
                        let field_idents1 = field_idents.clone();
                        tokens_cs = quote! {
                            #tokens_cs
                            #cs_ident {
                                #(#field_idents1: #field_types),*
                            },
                        };
                        let field_idents1 = field_idents.clone();
                        let field_idents2 = field_idents.clone();
                        let cs_id = cs.id;
                        tokens_cs_ser = quote! {
                            #tokens_cs_ser
                            &#ty_ident::#cs_ident{#(ref #field_idents1),*} => {
                                (#cs_id).serialize(out)?;
                                #(#field_idents2.serialize(out)?;)*
                            },
                        };
                        tokens_cs_deser = quote! {
                            #tokens_cs_deser
                            #cs_id => {
                                #ty_ident::#cs_ident {
                                    #(#field_idents: TLType::deserialize(input)?),*
                                }
                            },
                        };
                    }
                    tokens_ty = quote! {
                        #tokens_ty
                        #[allow(dead_code)]
                        #[derive(Clone,Debug,PartialEq)]
                        pub enum #ty_ident {
                            #tokens_cs
                        }
                        impl TLType for #ty_ident {
                            fn serialize<W: Write>(&self,out: &mut W) -> Result<()> {
                                match self {
                                    #tokens_cs_ser
                                };
                                Ok(())
                            }
                            fn deserialize<R: Read>(input: &mut R) -> Result<Self> {
                                let id = i32::deserialize(input)?;
                                let res = match id {
                                    #tokens_cs_deser
                                    _ => bail!(ErrorKind::Deserialize(#ty_name.into()))
                                };
                                Ok(res)
                            }
                        }
                    };
                }
                tokens = if ns_name == "" {
                    quote! {
                        #tokens
                        #tokens_ty
                    }
                } else {
                    quote! {
                        #tokens
                        mod #ns_ident {
                            use super::errors::*;
                            use super::{TLType,Write,Read};
                            #tokens_ty
                        }
                    }
                };
            }
            tokens
        }
    }
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("constructors.rs");
    // This test file is for easy inspection during debugging
    //let test_path = Path::new("/tmp").join("constructors.rs");
    let mut out = File::create(&dest_path).unwrap();
    let mut test = File::create(&test_path).unwrap();

    let mut f = File::open("schema.json").unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    let s = schema::Schema::new(s.as_str());
    let a =  ast::Ast::new(&s);
    let tokens = a.compile();

    // The `\n` is added for improved error highlighting when the code is wrong.
    // Otherwise everything is on the same line and rustc is confused
    out.write_all(tokens.as_str().replace("} ","}\n").as_bytes()).unwrap();
    //test.write_all(tokens.as_str().replace("} ","}\n").as_bytes()).unwrap();
}
