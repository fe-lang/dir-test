use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use proc_macro2::Span;
use quote::quote;

use syn::Token;

type Error = syn::Error;
type Result<T> = syn::Result<T>;

#[proc_macro_attribute]
pub fn dir_test(
    attrs: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr = syn::parse_macro_input!(attrs as DirTestArg);
    let func = syn::parse_macro_input!(item as syn::ItemFn);

    match TestBuilder::new(attr, func).build() {
        Ok((tests, func)) => quote! {
            #func
            #tests
        }
        .into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct TestBuilder {
    dir_test_arg: DirTestArg,
    func: syn::ItemFn,
    test_attrs: Vec<syn::Attribute>,
}

impl TestBuilder {
    fn new(dir_test_arg: DirTestArg, func: syn::ItemFn) -> Self {
        Self {
            dir_test_arg,
            func,
            test_attrs: vec![],
        }
    }

    fn build(mut self) -> Result<(proc_macro2::TokenStream, syn::ItemFn)> {
        self.extract_test_attrs()?;

        let mut pattern = self.dir_test_arg.resolve_dir()?;

        pattern.push(
            &self
                .dir_test_arg
                .glob
                .clone()
                .map_or_else(|| "*".to_string(), |g| g.value()),
        );

        let paths = glob::glob(&pattern.to_string_lossy()).map_err(|e| {
            Error::new_spanned(
                self.dir_test_arg.glob.clone().unwrap(),
                format!("failed to resolve glob pattern: {e}"),
            )
        })?;

        let mut tests = vec![];
        for entry in paths.filter_map(|p| p.ok()) {
            if !entry.is_file() {
                continue;
            }

            tests.push(self.build_test(&entry)?);
        }

        Ok((
            quote! {
                #(#tests)*
            },
            self.func,
        ))
    }

    fn build_test(&self, file_path: &Path) -> Result<proc_macro2::TokenStream> {
        let test_func = &self.func.sig.ident;
        let test_name = self.test_name(test_func.to_string(), file_path)?;
        let file_path_str = file_path.to_string_lossy();
        let return_ty = &self.func.sig.output;
        let test_attrs = &self.test_attrs;

        let loader = match self.dir_test_arg.loader {
            Some(ref loader) => quote! {#loader},
            None => quote! {::core::include_str!},
        };

        Ok(quote! {
            #(#test_attrs)*
            #[test]
            fn #test_name() #return_ty {
                #test_func(::dir_test::Fixture::new(#loader(#file_path_str), #file_path_str))
            }
        })
    }

    fn test_name(&self, test_func_name: String, fixture_path: &Path) -> Result<syn::Ident> {
        assert!(fixture_path.is_file());

        let dir_path = self.dir_test_arg.resolve_dir()?;
        let rel_path = fixture_path.strip_prefix(dir_path).unwrap();
        assert!(rel_path.is_relative());

        let mut test_name = test_func_name;
        test_name.push_str("__");

        let components: Vec<_> = rel_path.iter().collect();

        for component in &components[0..components.len() - 1] {
            let component = component
                .to_string_lossy()
                .replace(|c: char| c.is_ascii_punctuation(), "_");
            test_name.push_str(&component);
            test_name.push('_');
        }

        test_name.push_str(
            &rel_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .replace(|c: char| c.is_ascii_punctuation(), "_"),
        );

        if let Some(postfix) = &self.dir_test_arg.postfix {
            test_name.push('_');
            test_name.push_str(&postfix.value());
        }

        Ok(make_ident(&test_name))
    }

    /// Extracts `#[dir_test_attr(...)]` from function attributes.
    fn extract_test_attrs(&mut self) -> Result<()> {
        let mut err = Ok(());
        self.func.attrs.retain(|attr| {
            if attr.path.is_ident("dir_test_attr") {
                err = err
                    .clone()
                    .and(attr.parse_args_with(|input: syn::parse::ParseStream| {
                        self.test_attrs
                            .extend(input.call(syn::Attribute::parse_outer)?);
                        if !input.is_empty() {
                            Err(Error::new(
                                input.span(),
                                "unexpected token after `dir_test_attr`",
                            ))
                        } else {
                            Ok(())
                        }
                    }));

                false
            } else {
                true
            }
        });

        err
    }
}

#[derive(Default)]
struct DirTestArg {
    dir: Option<syn::LitStr>,
    glob: Option<syn::LitStr>,
    postfix: Option<syn::LitStr>,
    loader: Option<syn::Path>,
}

impl DirTestArg {
    fn resolve_dir(&self) -> Result<PathBuf> {
        let Some(dir) = &self.dir else {
            return Err(Error::new(Span::call_site(), "`dir` is required"));
        };

        let resolved = self.resolve_path(Path::new(&dir.value()))?;

        if !resolved.is_absolute() {
            return Err(Error::new_spanned(
                dir.clone(),
                format!("`{}` is not an absolute path", resolved.display()),
            ));
        } else if !resolved.exists() {
            return Err(Error::new_spanned(
                dir.clone(),
                format!("`{}` does not exist", resolved.display()),
            ));
        } else if !resolved.is_dir() {
            return Err(Error::new_spanned(
                dir.clone(),
                format!("`{}` is not a directory", resolved.display()),
            ));
        }

        Ok(resolved)
    }

    fn resolve_path(&self, path: &Path) -> Result<PathBuf> {
        let mut resolved = PathBuf::new();
        for component in path {
            resolved.push(self.resolve_component(component)?);
        }
        Ok(resolved)
    }

    fn resolve_component(&self, component: &OsStr) -> Result<PathBuf> {
        if component.to_string_lossy().starts_with('$') {
            let env_var = &component.to_string_lossy()[1..];
            let env_var_value = std::env::var(env_var).map_err(|e| {
                Error::new_spanned(
                    self.dir.clone().unwrap(),
                    format!("failed to resolve env var `{env_var}`: {e}"),
                )
            })?;
            let resolved = self.resolve_path(Path::new(&env_var_value))?;
            Ok(resolved)
        } else {
            Ok(Path::new(&component).into())
        }
    }
}

impl syn::parse::Parse for DirTestArg {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut dir_test_attr = DirTestArg::default();
        let mut visited_args: HashSet<String> = HashSet::new();

        while !input.is_empty() {
            let arg = input.parse::<syn::Ident>()?;
            if visited_args.contains(&arg.to_string()) {
                return Err(Error::new_spanned(
                    arg.clone(),
                    format!("duplicated arg `{arg}`"),
                ));
            }

            match arg.to_string().as_str() {
                "dir" => {
                    input.parse::<Token![:]>()?;
                    dir_test_attr.dir = Some(input.parse()?);
                }

                "glob" => {
                    input.parse::<Token![:]>()?;
                    dir_test_attr.glob = Some(input.parse()?);
                }

                "postfix" => {
                    input.parse::<Token![:]>()?;
                    dir_test_attr.postfix = Some(input.parse()?);
                }

                "loader" => {
                    input.parse::<Token![:]>()?;
                    dir_test_attr.loader = Some(input.parse()?);
                }

                _ => {
                    return Err(Error::new_spanned(
                        arg.clone(),
                        format!("unknown arg `{arg}`"),
                    ))
                }
            };

            visited_args.insert(arg.to_string());
            input.parse::<syn::Token![,]>().ok();
        }

        Ok(dir_test_attr)
    }
}

fn make_ident(name: &str) -> syn::Ident {
    if is_keyword(name) {
        syn::Ident::new_raw(name, Span::call_site())
    } else {
        syn::Ident::new(name, Span::call_site())
    }
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum "
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}
