use std::{
    collections::HashSet,
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
    let attr = syn::parse_macro_input!(attrs as DirTestAttr);
    let func = syn::parse_macro_input!(item as syn::ItemFn);

    match TestBuilder::new(attr, &func).build() {
        Ok(tests) => quote! {
            #func
            #tests
        }
        .into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct TestBuilder<'a> {
    attr: DirTestAttr,
    func: &'a syn::ItemFn,
}

impl<'a> TestBuilder<'a> {
    fn new(attr: DirTestAttr, func: &'a syn::ItemFn) -> Self {
        Self { attr, func }
    }

    fn build(self) -> Result<proc_macro2::TokenStream> {
        let mut pattern = self.attr.resolve_dir()?;

        pattern.push(
            &self
                .attr
                .glob
                .clone()
                .map_or_else(|| "*".to_string(), |g| g.value()),
        );

        let paths = glob::glob(&pattern.to_string_lossy()).map_err(|e| {
            Error::new_spanned(
                self.attr.glob.clone().unwrap(),
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

        Ok(quote! {
            #(#tests)*
        })
    }

    fn build_test(&self, file_path: &Path) -> Result<proc_macro2::TokenStream> {
        let test_attrs = &self.attr.test_attrs;
        let test_name = self.test_name(file_path)?;
        let file_path_str = file_path.to_string_lossy();
        let test_func = &self.func.sig.ident;

        Ok(quote! {
            #(#test_attrs)*
            #[test]
            fn #test_name() {
                let __dir_test_fixture = ::dir_test::Fixture::new(::std::include_str!(#file_path_str), ::std::path::Path::new(#file_path_str));
                #test_func(__dir_test_fixture);
            }
        })
    }

    fn test_name(&self, fixture_path: &Path) -> Result<syn::Ident> {
        assert!(fixture_path.is_file());

        let dir_path = self.attr.resolve_dir()?;
        let rel_path = fixture_path.strip_prefix(dir_path).unwrap();
        assert!(rel_path.is_relative());

        let mut test_name = String::new();
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

        if let Some(postfix) = &self.attr.postfix {
            test_name.push('_');
            test_name.push_str(&postfix.value());
        }

        Ok(syn::Ident::new(&test_name, Span::call_site()))
    }
}

#[derive(Default)]
struct DirTestAttr {
    dir: Option<syn::LitStr>,
    glob: Option<syn::LitStr>,
    postfix: Option<syn::LitStr>,
    test_attrs: Vec<syn::Attribute>,
}

impl DirTestAttr {
    fn resolve_dir(&self) -> Result<PathBuf> {
        let Some(dir) = &self.dir else {
            return Err(Error::new(Span::call_site(), "`dir` is required"));
        };

        let mut resolved = PathBuf::new();
        for component in Path::new(&dir.value()) {
            if component.to_string_lossy().starts_with('$') {
                let env_var = &component.to_string_lossy()[1..];
                let env_var_value = std::env::var(env_var).map_err(|e| {
                    Error::new_spanned(
                        dir.clone(),
                        format!("failed to resolve env var `{env_var}`: {e}"),
                    )
                })?;
                resolved.push(env_var_value);
            } else {
                resolved.push(component);
            }
        }

        if !resolved.is_dir() {
            return Err(Error::new_spanned(
                dir.clone(),
                format!("`{}` is not a directory", resolved.display()),
            ));
        } else if !resolved.exists() {
            return Err(Error::new_spanned(
                dir.clone(),
                format!("`{}` does not exist", resolved.display()),
            ));
        } else if !resolved.is_absolute() {
            return Err(Error::new_spanned(
                dir.clone(),
                format!("`{}` is not an absolute path", resolved.display()),
            ));
        }

        Ok(resolved)
    }
}

impl syn::parse::Parse for DirTestAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut dir_test_attr = DirTestAttr::default();
        let mut visited_args: HashSet<String> = HashSet::new();
        dir_test_attr.test_attrs = input.call(syn::Attribute::parse_outer)?;

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
