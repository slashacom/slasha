use axum::{
    Json,
    extract::{FromRequest, Request},
};
use garde::Validate;
use serde::de::DeserializeOwned;

use crate::HttpError;

pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    T::Context: Default,
    S: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state)
            .await
            .map_err(|error| HttpError::bad_request(error.body_text()))?;
        value.validate()?;
        Ok(Self(value))
    }
}
