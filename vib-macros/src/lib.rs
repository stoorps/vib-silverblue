
use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;
use syn::{
    parse_macro_input, DeriveInput, Lit, Meta,
    NestedMeta,  Type,
};

#[proc_macro_attribute]
pub fn build_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {

    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = input_fn.sig.ident.clone();
    let mut module_type: Option<Type> = None;

    // Find the module type (the one that's NOT Recipe)
    for input in &input_fn.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if !is_recipe_type(&pat_type.ty) { // Check if it's NOT Recipe
                module_type = Some(*pat_type.ty.clone());
                break;
            }
        }
    }

    let module_type = module_type.expect("A parameter (other than Recipe) must be present, and must implement the `module_info` macro");


    let expanded = quote! {
        #input_fn
    
        #[no_mangle]
        pub unsafe extern "C" fn BuildModule(
            module_interface: *const std::ffi::c_char,
            recipe_interface: *const std::ffi::c_char,
        ) -> *mut std::ffi::c_char {
            use std::ffi::{CStr, CString}; // Import necessary modules
    
            let recipe = CStr::from_ptr(recipe_interface);
            let module = CStr::from_ptr(module_interface);
    
            let module = String::from_utf8_lossy(module.to_bytes()).to_string();
            let recipe = String::from_utf8_lossy(recipe.to_bytes()).to_string();
    
            let module: #module_type = match serde_json::from_str(&module) {
                Ok(v) => v,
                Err(error) => {
                    let error_message = format!("ERROR: {}", error); // Format error message *outside*
                    return CString::new(error_message).unwrap().into_raw();
                }
            };
    
            let recipe: Recipe = match serde_json::from_str(&recipe) {
                Ok(v) => v,
                Err(error) => {
                    let error_message = format!("ERROR: {}", error); // Format error message *outside*
                    return CString::new(error_message).unwrap().into_raw();
                }
            };
    
            let cmd = #fn_name(module, recipe);
            let rtrn = CString::new(cmd).expect("ERROR: CString::new failed");
            rtrn.into_raw()
        }
    };

    expanded.into()
}


fn is_recipe_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(ident) = path.path.get_ident() {
            return ident == "Recipe"; // Or whatever your Recipe type is named
        }
    }
    false
}


#[proc_macro_attribute]
pub fn module_info(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attrs = parse_macro_input!(attr as syn::AttributeArgs);

    let mut name = String::new();
    let mut module_type = String::new();
    let mut use_container_cmds = String::new();

    for attr in attrs {
        match attr {
            NestedMeta::Meta(Meta::NameValue(nv)) => {
                let ident = nv.path.get_ident().unwrap().to_string();
                let lit = nv.lit;
                match ident.as_str() {
                    "name" => name = get_lit_string(&lit),
                    "module_type" => module_type = get_lit_string(&lit),
                    "use_container_cmds" => use_container_cmds = get_lit_string(&lit),
                    _ => panic!("Unknown attribute: {}", ident),
                }
            }
            _ => panic!("Invalid attribute format"),
        }
    }

    let generated_code = quote! {
        #input

        
        #[no_mangle]
        pub unsafe extern "C" fn PlugInfo() -> *mut c_char {

            let json_str = format!("\"Name\":\"{}\",\"Type\":\"{}\",\"Usecontainercmds\":{}", #name, #module_type, #use_container_cmds);
            let rtrn = CString::new(json_str).unwrap();
        
            
            rtrn.into_raw()
        }
    };

    generated_code.into()
}

fn get_lit_string(lit: &Lit) -> String {
    match lit {
        Lit::Str(s) => s.value(),
        _ => panic!("Attribute value must be a string literal"),
    }
}