// Copyright 2023-2025 The Android Open Source Project

// =============================================================================
// CLIENT METHOD MACRO
// =============================================================================

/// Generate client methods with oneshot channel boilerplate.
/// Client methods convert domain errors to String for API simplicity.
#[macro_export]
macro_rules! client_method {
    // Pattern for methods WITH arguments
    ($client:ty => fn $method:ident($($param:ident: $param_type:ty),*) -> $return_type:ty as $request:ident::$variant:ident) => {
        impl $client {
            pub async fn $method(&self, $($param: $param_type),*) -> std::result::Result<$return_type, ClientError> {
                let (respond_to, response) = oneshot::channel();
                self.sender.send($request::$variant {
                    $($param,)*
                    respond_to,
                }).await.map_err(|e| ClientError::Send(e.to_string()))?;
                Ok(response.await??)
            }
        }
    };

    // Pattern for methods WITHOUT arguments
    ($client:ty => fn $method:ident() -> $return_type:ty as $request:ident::$variant:ident) => {
        impl $client {
            pub async fn $method(&self) -> std::result::Result<$return_type, ClientError> {
                let (respond_to, response) = oneshot::channel();
                self.sender.send($request::$variant { respond_to })
                    .await
                    .map_err(|e| ClientError::Send(e.to_string()))?;
                Ok(response.await??)
            }
        }
    };
}
