use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error};

/// Automatically register a task type with the task registry.
///
/// This derive macro generates code that uses the inventory pattern to automatically
/// register task types at runtime. The task will be registered using the name returned
/// by its `name()` method.
///
/// This macro uses the inventory pattern to automatically register task types at runtime.
///
/// # Example
///
/// ```rust
/// #[derive(Debug, Serialize, Deserialize, AutoRegisterTask)]
/// struct MyTask {
///     data: String,
/// }
///
/// #[async_trait]
/// impl Task for MyTask {
///     async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
///         // task implementation  
///         use serde::Serialize;
///         #[derive(Serialize)]
///         struct Response { status: String }
///         let response = Response { status: "completed".to_string() };
///         Ok(rmp_serde::to_vec(&response)?)
///     }
///     
///     fn name(&self) -> &str {
///         "my_task"
///     }
/// }
/// ```
#[proc_macro_derive(AutoRegisterTask)]
pub fn auto_register_task(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    impl_auto_register_task(&input).unwrap_or_else(|err| err.to_compile_error().into())
}

fn impl_auto_register_task(input: &DeriveInput) -> Result<TokenStream, Error> {
    let type_name = &input.ident;

    // Generate the inventory submission code
    let expanded = quote! {
        // Submit this task type to the inventory for automatic registration
        ::rust_task_queue::inventory::submit! {
            ::rust_task_queue::TaskRegistration {
                type_name: stringify!(#type_name),
                register_fn: |registry: &::rust_task_queue::TaskRegistry| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    // We need to get the task name from the type
                    // For this we create a temporary instance with Default::default()
                    // This is a limitation but keeps the implementation simple
                    let temp_instance = <#type_name as Default>::default();
                    let task_name = temp_instance.name();

                    registry.register_with_name::<#type_name>(task_name)
                },
            }
        }
    };

    Ok(expanded.into())
}

/// Attribute macro for registering tasks with a custom name.
///
/// This is useful when you want to register a task with a specific name
/// rather than using the name returned by the `name()` method.
///
/// # Example
///
/// ```rust
/// #[register_task("custom_task_name")]
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyTask {
///     data: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn register_task(args: TokenStream, input: TokenStream) -> TokenStream {
    let task_name =
        if args.is_empty() {
            None
        } else {
            // Parse the task name from the attribute arguments
            match syn::parse::<syn::LitStr>(args.clone()) {
            Ok(lit) => Some(lit.value()),
            Err(_) => return Error::new_spanned(
                proc_macro2::TokenStream::from(args),
                "Expected string literal for task name, e.g., #[register_task(\"my_task_name\")]"
            ).to_compile_error().into(),
        }
        };

    let input = parse_macro_input!(input as DeriveInput);

    impl_register_task_with_name(&input, task_name)
        .unwrap_or_else(|err| err.to_compile_error().into())
}

fn impl_register_task_with_name(
    input: &DeriveInput,
    custom_name: Option<String>,
) -> Result<TokenStream, Error> {
    let type_name = &input.ident;

    let registration_code = if let Some(name) = custom_name {
        // Use the custom name provided in the attribute
        quote! {
            ::rust_task_queue::inventory::submit! {
                ::rust_task_queue::TaskRegistration {
                    type_name: stringify!(#type_name),
                    register_fn: |registry: &::rust_task_queue::TaskRegistry| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                        registry.register_with_name::<#type_name>(#name)
                    },
                }
            }
        }
    } else {
        // Use the name from the Task::name() method
        quote! {
            ::rust_task_queue::inventory::submit! {
                ::rust_task_queue::TaskRegistration {
                    type_name: stringify!(#type_name),
                    register_fn: |registry: &::rust_task_queue::TaskRegistry| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                        let temp_instance = <#type_name as Default>::default();
                        let task_name = temp_instance.name();
                        registry.register_with_name::<#type_name>(task_name)
                    },
                }
            }
        }
    };

    // Include the original struct/enum definition along with the registration
    let expanded = quote! {
        #input

        #registration_code
    };

    Ok(expanded.into())
}
